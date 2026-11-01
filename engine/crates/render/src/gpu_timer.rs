use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

const READBACK_SLOTS: usize = 3;
const RESULT_BYTES: u64 = 16;

struct ReadbackSlot {
    buffer: wgpu::Buffer,
    busy: Arc<AtomicBool>,
}

#[derive(Default)]
struct Samples {
    total_ns: AtomicU64,
    count: AtomicU32,
}

pub(crate) struct GpuTimer {
    query_set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    slots: Vec<ReadbackSlot>,
    next: usize,
    period_ns: f32,
    samples: Arc<Samples>,
}

impl GpuTimer {
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("frame time"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame time resolve"),
            size: RESULT_BYTES,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let slots = (0..READBACK_SLOTS)
            .map(|_| ReadbackSlot {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("frame time readback"),
                    size: RESULT_BYTES,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                busy: Arc::new(AtomicBool::new(false)),
            })
            .collect();
        GpuTimer {
            query_set,
            resolve,
            slots,
            next: 0,
            period_ns: queue.get_timestamp_period(),
            samples: Arc::new(Samples::default()),
        }
    }

    pub(crate) fn acquire(&mut self) -> Option<usize> {
        let index = self.next;
        if self.slots[index].busy.load(Ordering::Acquire) {
            return None;
        }
        self.next = (index + 1) % self.slots.len();
        Some(index)
    }

    pub(crate) fn pass_writes(&self) -> wgpu::RenderPassTimestampWrites<'_> {
        wgpu::RenderPassTimestampWrites {
            query_set: &self.query_set,
            beginning_of_pass_write_index: Some(0),
            end_of_pass_write_index: Some(1),
        }
    }

    pub(crate) fn begin_writes(&self) -> wgpu::RenderPassTimestampWrites<'_> {
        wgpu::RenderPassTimestampWrites {
            query_set: &self.query_set,
            beginning_of_pass_write_index: Some(0),
            end_of_pass_write_index: None,
        }
    }

    pub(crate) fn end_writes(&self) -> wgpu::RenderPassTimestampWrites<'_> {
        wgpu::RenderPassTimestampWrites {
            query_set: &self.query_set,
            beginning_of_pass_write_index: None,
            end_of_pass_write_index: Some(1),
        }
    }

    pub(crate) fn resolve(&self, encoder: &mut wgpu::CommandEncoder, slot: usize) {
        encoder.resolve_query_set(&self.query_set, 0..2, &self.resolve, 0);
        encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.slots[slot].buffer, 0, RESULT_BYTES);
    }

    pub(crate) fn read_back(&self, slot: usize) {
        let slot = &self.slots[slot];
        slot.busy.store(true, Ordering::Release);
        let buffer = slot.buffer.clone();
        let busy = slot.busy.clone();
        let samples = self.samples.clone();
        let period_ns = f64::from(self.period_ns);
        slot.buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                if result.is_ok() {
                    {
                        let Ok(view) = buffer.slice(..).get_mapped_range() else {
                            busy.store(false, Ordering::Release);
                            return;
                        };
                        let word =
                            |i: usize| u64::from_le_bytes(view[i..i + 8].try_into().unwrap());
                        let (begin, end) = (word(0), word(8));
                        if end > begin {
                            let ns = ((end - begin) as f64 * period_ns) as u64;
                            samples.total_ns.fetch_add(ns, Ordering::Relaxed);
                            samples.count.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    buffer.unmap();
                }
                busy.store(false, Ordering::Release);
            });
    }

    pub(crate) fn take_samples(&self) -> Option<(f64, u32)> {
        let count = self.samples.count.swap(0, Ordering::Relaxed);
        let total_ns = self.samples.total_ns.swap(0, Ordering::Relaxed);
        (count > 0).then(|| (total_ns as f64 / 1e6, count))
    }
}
