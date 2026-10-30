use mesh::mesh::{SLOT_BYTES, slot_bytes};

use crate::arena::{grow_buffer, grown_capacity};

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

    pub(crate) fn used(&self) -> u32 {
        self.next - self.free.len() as u32
    }

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

pub(crate) struct SlotBuffer {
    buffer: wgpu::Buffer,
    slots: SlotAllocator,
    max_slots: u32,
}

const INITIAL_SLOTS: u32 = 4096;

impl SlotBuffer {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let max_slots = (device.limits().max_buffer_size / SLOT_BYTES as u64).min(1 << 31) as u32;
        let slots = INITIAL_SLOTS.min(max_slots);
        SlotBuffer {
            buffer: Self::create_buffer(device, u64::from(slots) * SLOT_BYTES as u64),
            slots: SlotAllocator::new(slots),
            max_slots,
        }
    }

    fn create_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh slots"),
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

    pub(crate) fn bytes(&self) -> u64 {
        self.buffer.size()
    }

    pub(crate) fn used_bytes(&self) -> u64 {
        u64::from(self.slots.used()) * SLOT_BYTES as u64
    }

    pub(crate) fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        origin: [f32; 3],
        first_word: u32,
    ) -> Result<u32, String> {
        let slot = match self.slots.alloc() {
            Some(slot) => slot,
            None => {
                let step = SLOT_BYTES as u64;
                let needed = (u64::from(self.slots.capacity()) + 1) * step;
                let bytes =
                    grown_capacity(self.bytes(), needed, u64::from(self.max_slots) * step, step)
                        .ok_or("mesh slot buffer is full")?;
                self.buffer = grow_buffer(device, queue, &self.buffer, bytes, "mesh slots");
                self.slots.grow((bytes / step) as u32);
                self.slots
                    .alloc()
                    .ok_or("mesh slot buffer: no slot after growth")?
            }
        };
        queue.write_buffer(
            &self.buffer,
            u64::from(slot) * SLOT_BYTES as u64,
            &slot_bytes(origin, first_word),
        );
        Ok(slot)
    }

    pub(crate) fn free(&mut self, slot: u32) {
        self.slots.free(slot);
    }
}

#[cfg(test)]
mod tests;
