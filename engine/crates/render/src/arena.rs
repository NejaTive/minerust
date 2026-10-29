use std::collections::{BTreeMap, BTreeSet};

use wgpu::util::DeviceExt;

pub(crate) struct RangeAllocator {
    capacity: u64,
    used: u64,
    align: u64,
    by_offset: BTreeMap<u64, u64>,
    by_size: BTreeSet<(u64, u64)>,
}

impl RangeAllocator {
    pub(crate) fn new(capacity: u64, align: u64) -> Self {
        debug_assert!(align.is_power_of_two() && capacity.is_multiple_of(align));
        let mut allocator = RangeAllocator {
            capacity: 0,
            used: 0,
            align,
            by_offset: BTreeMap::new(),
            by_size: BTreeSet::new(),
        };
        allocator.grow(capacity);
        allocator
    }

    fn round_up(&self, size: u64) -> u64 {
        size.next_multiple_of(self.align)
    }

    pub(crate) fn capacity(&self) -> u64 {
        self.capacity
    }

    pub(crate) fn used(&self) -> u64 {
        self.used
    }

    #[cfg(test)]
    pub(crate) fn free_range_count(&self) -> usize {
        self.by_offset.len()
    }

    #[cfg(test)]
    pub(crate) fn largest_free(&self) -> u64 {
        self.by_size.last().map_or(0, |&(size, _)| size)
    }

    pub(crate) fn alloc(&mut self, size: u64) -> Option<u64> {
        let size = self.round_up(size.max(1));
        let &(free_size, offset) = self.by_size.range((size, 0)..).next()?;
        self.remove_free(offset, free_size);
        if free_size > size {
            self.insert_free(offset + size, free_size - size);
        }
        self.used += size;
        Some(offset)
    }

    pub(crate) fn free(&mut self, offset: u64, size: u64) {
        let size = self.round_up(size.max(1));
        debug_assert!(offset + size <= self.capacity);
        self.used -= size;
        let (mut start, mut len) = (offset, size);
        if let Some((&prev, &prev_len)) = self.by_offset.range(..offset).next_back()
            && prev + prev_len == start
        {
            self.remove_free(prev, prev_len);
            start = prev;
            len += prev_len;
        }
        if let Some(&next_len) = self.by_offset.get(&(offset + size)) {
            self.remove_free(offset + size, next_len);
            len += next_len;
        }
        self.insert_free(start, len);
    }

    pub(crate) fn grow(&mut self, new_capacity: u64) {
        debug_assert!(new_capacity >= self.capacity && new_capacity.is_multiple_of(self.align));
        let old = self.capacity;
        self.capacity = new_capacity;
        if new_capacity == old {
            return;
        }
        let (mut start, mut len) = (old, new_capacity - old);
        if let Some((&prev, &prev_len)) = self.by_offset.range(..old).next_back()
            && prev + prev_len == old
        {
            self.remove_free(prev, prev_len);
            start = prev;
            len += prev_len;
        }
        self.insert_free(start, len);
    }

    fn insert_free(&mut self, offset: u64, size: u64) {
        self.by_offset.insert(offset, size);
        self.by_size.insert((size, offset));
    }

    fn remove_free(&mut self, offset: u64, size: u64) {
        self.by_offset.remove(&offset);
        self.by_size.remove(&(size, offset));
    }
}

pub(crate) fn grown_capacity(current: u64, needed: u64, max: u64, align: u64) -> Option<u64> {
    let max = max - max % align;
    let mut capacity = current.max(align);
    while capacity < needed {
        capacity += (capacity / 4).next_multiple_of(align);
    }
    let capacity = capacity.min(max);
    (capacity >= needed && capacity > current).then_some(capacity)
}

pub(crate) fn grow_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    old: &wgpu::Buffer,
    capacity: u64,
    label: &str,
) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: capacity,
        usage: old.usage(),
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("grow buffer"),
    });
    encoder.copy_buffer_to_buffer(old, 0, &buffer, 0, old.size());
    queue.submit([encoder.finish()]);
    buffer
}

pub(crate) struct QuadArena {
    buffer: wgpu::Buffer,
    table: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    allocator: RangeAllocator,
    max_size: u64,
}

pub(crate) const WORD: u64 = 4;

const INITIAL_SIZE: u64 = 16 << 20;

impl QuadArena {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let limits = device.limits();
        let max_size = limits
            .max_storage_buffer_binding_size
            .min(limits.max_buffer_size);
        let size = INITIAL_SIZE.min(max_size - max_size % WORD);
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("quad arena"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let buffer = Self::create_buffer(device, size);
        let table = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad templates"),
            contents: &mesh::templates::table_bytes(),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        QuadArena {
            bind_group: Self::create_bind_group(device, &bind_group_layout, &buffer, &table),
            buffer,
            table,
            bind_group_layout,
            allocator: RangeAllocator::new(size, WORD),
            max_size,
        }
    }

    fn create_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad arena"),
            size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    }

    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        buffer: &wgpu::Buffer,
        table: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("quad arena"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: table.as_entire_binding(),
                },
            ],
        })
    }

    pub(crate) fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub(crate) fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub(crate) fn capacity(&self) -> u64 {
        self.allocator.capacity()
    }

    pub(crate) fn used(&self) -> u64 {
        self.allocator.used()
    }

    pub(crate) fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
    ) -> Result<u64, String> {
        let size = bytes.len() as u64;
        let offset = match self.allocator.alloc(size) {
            Some(offset) => offset,
            None => {
                let needed = self.allocator.capacity() + size;
                let capacity =
                    grown_capacity(self.allocator.capacity(), needed, self.max_size, WORD)
                        .ok_or_else(|| {
                            format!(
                                "quad arena is full: {} MiB used of the {} MiB buffer limit",
                                self.allocator.used() >> 20,
                                self.max_size >> 20
                            )
                        })?;
                self.buffer = grow_buffer(device, queue, &self.buffer, capacity, "quad arena");
                self.bind_group = Self::create_bind_group(
                    device,
                    &self.bind_group_layout,
                    &self.buffer,
                    &self.table,
                );
                self.allocator.grow(capacity);
                self.allocator
                    .alloc(size)
                    .ok_or("quad arena: no room after growth")?
            }
        };
        queue.write_buffer(&self.buffer, offset, bytes);
        Ok(offset)
    }

    pub(crate) fn free(&mut self, offset: u64, size: u64) {
        self.allocator.free(offset, size);
    }
}

#[cfg(test)]
mod tests;
