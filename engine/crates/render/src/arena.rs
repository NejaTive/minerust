use std::collections::{BTreeMap, BTreeSet};

/// Аллокатор диапазонов в байтах без GPU: свободные диапазоны хранятся по смещению (слияние
/// соседей при освобождении) и по размеру (best fit за O(log n)). Buddy не годится: размеры
/// мешей секций различаются на порядки, округление до степени двойки съело бы запас.
pub(crate) struct RangeAllocator {
    capacity: u64,
    used: u64,
    align: u64,
    by_offset: BTreeMap<u64, u64>,
    by_size: BTreeSet<(u64, u64)>,
}

impl RangeAllocator {
    /// `align` — степень двойки; смещения и размеры диапазонов кратны ей.
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

    /// Занято байт (с округлением диапазонов до `align`).
    pub(crate) fn used(&self) -> u64 {
        self.used
    }

    /// Свободных диапазонов; много — значит арена фрагментирована.
    #[cfg(test)]
    pub(crate) fn free_range_count(&self) -> usize {
        self.by_offset.len()
    }

    /// Наибольший свободный диапазон, байт.
    #[cfg(test)]
    pub(crate) fn largest_free(&self) -> u64 {
        self.by_size.last().map_or(0, |&(size, _)| size)
    }

    /// Смещение диапазона не меньше `size` байт или `None`, если подходящего нет.
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

    /// Возвращает диапазон, выданный `alloc(size)` по `offset`.
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

    /// Увеличивает ёмкость до `new_capacity`; новый хвост сливается со свободным концом.
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

/// Новая ёмкость при росте: +25% (но не меньше `needed`), не выше `max`. Удвоение
/// оставляло арену на 2× больше занятого после перелётов, а она не сжимается.
/// `None` — предел достигнут, и `needed` не помещается.
pub(crate) fn grown_capacity(current: u64, needed: u64, max: u64, align: u64) -> Option<u64> {
    let max = max - max % align;
    let mut capacity = current.max(align);
    while capacity < needed {
        capacity += (capacity / 4).next_multiple_of(align);
    }
    let capacity = capacity.min(max);
    (capacity >= needed && capacity > current).then_some(capacity)
}

/// Буфер на `capacity` байт с содержимым `old` в начале: рост арены и буфера углов.
/// Копирование сразу уходит в очередь: записи `write_buffer` после него видят новый буфер,
/// а кадр, записанный до роста, ссылается на старый, который живёт до конца его выполнения.
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

/// Один буфер вершин на все меши секций: диапазоны выдаёт [`RangeAllocator`].
pub(crate) struct VertexArena {
    buffer: wgpu::Buffer,
    allocator: RangeAllocator,
    max_size: u64,
}

/// Размер вершины, байт: смещение диапазона делится на него в `base_vertex`.
pub(crate) const VERTEX_SIZE: u64 = size_of::<mesh::mesh::Vertex>() as u64;

/// Начальный размер арены. Лес после 14b — ~31 МиБ мешей (дальность 12), так что
/// обычная игра обходится без роста: он дорогой (копия всего буфера).
const INITIAL_SIZE: u64 = 32 << 20;

impl VertexArena {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let max_size = device.limits().max_buffer_size;
        let size = INITIAL_SIZE.min(max_size - max_size % VERTEX_SIZE);
        VertexArena {
            buffer: Self::create_buffer(device, size),
            allocator: RangeAllocator::new(size, VERTEX_SIZE),
            max_size,
        }
    }

    fn create_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex arena"),
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

    pub(crate) fn capacity(&self) -> u64 {
        self.allocator.capacity()
    }

    pub(crate) fn used(&self) -> u64 {
        self.allocator.used()
    }

    /// Записывает `bytes` (кратно размеру вершины) и возвращает смещение. Нет места —
    /// арена растёт на 25%; на пределе `max_buffer_size` возвращает ошибку.
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
                // Свободного места в сумме может хватать, но не одним куском; растём
                // на 25% и не меньше чем на весь меш, а не склеиваем дыры.
                let needed = self.allocator.capacity() + size;
                let capacity = grown_capacity(
                    self.allocator.capacity(),
                    needed,
                    self.max_size,
                    VERTEX_SIZE,
                )
                .ok_or_else(|| {
                    format!(
                        "vertex arena is full: {} MiB used of the {} MiB buffer limit",
                        self.allocator.used() >> 20,
                        self.max_size >> 20
                    )
                })?;
                self.buffer = grow_buffer(device, queue, &self.buffer, capacity, "vertex arena");
                self.allocator.grow(capacity);
                self.allocator
                    .alloc(size)
                    .ok_or("vertex arena: no room after growth")?
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
