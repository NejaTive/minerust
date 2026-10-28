use mesh::mesh::{ORIGIN_BYTES, origin_bytes};

use crate::arena::{grow_buffer, grown_capacity};

/// Номера слотов без GPU: освобождённые переиспользуются раньше новых.
pub(crate) struct SlotAllocator {
    capacity: u32,
    next: u32,
    free: Vec<u32>,
}

impl SlotAllocator {
    pub(crate) fn new(capacity: u32) -> Self {
        SlotAllocator {
            capacity,
            next: 0,
            free: Vec::new(),
        }
    }

    pub(crate) fn capacity(&self) -> u32 {
        self.capacity
    }

    /// Занято слотов.
    pub(crate) fn used(&self) -> u32 {
        self.next - self.free.len() as u32
    }

    /// `None`, если слотов не осталось: нужен [`grow`](Self::grow).
    pub(crate) fn alloc(&mut self) -> Option<u32> {
        if let Some(slot) = self.free.pop() {
            return Some(slot);
        }
        (self.next < self.capacity).then(|| {
            self.next += 1;
            self.next - 1
        })
    }

    pub(crate) fn free(&mut self, slot: u32) {
        debug_assert!(slot < self.next && !self.free.contains(&slot));
        self.free.push(slot);
    }

    pub(crate) fn grow(&mut self, capacity: u32) {
        self.capacity = self.capacity.max(capacity);
    }
}

/// Мировые углы секций одним буфером экземпляров: слот секции — `first_instance`
/// отрисовки ([`mesh::origin_layout`]), так что буфер ставится раз на проход.
pub(crate) struct OriginBuffer {
    buffer: wgpu::Buffer,
    slots: SlotAllocator,
    max_slots: u32,
}

/// Секций с мешем при дальности 12 — около 1900, при максимальной — вдвое-втрое больше;
/// 2048 слотов — 32 КиБ, рост на 25% дешёв.
const INITIAL_SLOTS: u32 = 2048;

impl OriginBuffer {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let max_slots = (device.limits().max_buffer_size / ORIGIN_BYTES as u64).min(1 << 31) as u32;
        let slots = INITIAL_SLOTS.min(max_slots);
        OriginBuffer {
            buffer: Self::create_buffer(device, u64::from(slots) * ORIGIN_BYTES as u64),
            slots: SlotAllocator::new(slots),
            max_slots,
        }
    }

    fn create_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("section origins"),
            size,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    }

    pub(crate) fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Размер буфера, байт.
    pub(crate) fn bytes(&self) -> u64 {
        self.buffer.size()
    }

    /// Занято байт слотами.
    pub(crate) fn used_bytes(&self) -> u64 {
        u64::from(self.slots.used()) * ORIGIN_BYTES as u64
    }

    /// Слот с записанным углом `origin`.
    pub(crate) fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        origin: [f32; 3],
    ) -> Result<u32, String> {
        let slot = match self.slots.alloc() {
            Some(slot) => slot,
            None => {
                let step = ORIGIN_BYTES as u64;
                let needed = (u64::from(self.slots.capacity()) + 1) * step;
                let bytes =
                    grown_capacity(self.bytes(), needed, u64::from(self.max_slots) * step, step)
                        .ok_or("section origin buffer is full")?;
                self.buffer = grow_buffer(device, queue, &self.buffer, bytes, "section origins");
                self.slots.grow((bytes / step) as u32);
                self.slots
                    .alloc()
                    .ok_or("section origin buffer: no slot after growth")?
            }
        };
        queue.write_buffer(
            &self.buffer,
            u64::from(slot) * ORIGIN_BYTES as u64,
            &origin_bytes(origin),
        );
        Ok(slot)
    }

    pub(crate) fn free(&mut self, slot: u32) {
        self.slots.free(slot);
    }
}

#[cfg(test)]
mod tests;
