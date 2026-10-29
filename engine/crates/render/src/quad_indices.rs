use wgpu::util::DeviceExt;

use crate::arena::grown_capacity;

pub(crate) const INDICES_PER_QUAD: u32 = 6;
const INDEX_BYTES: u64 = size_of::<u32>() as u64;
const QUAD_INDEX_BYTES: u64 = INDICES_PER_QUAD as u64 * INDEX_BYTES;
const INITIAL_QUADS: u32 = 4096;

pub(crate) fn quad_indices(quads: u32) -> Vec<u32> {
    (0..quads)
        .flat_map(|q| [0, 1, 2, 0, 2, 3].map(|i| 4 * q + i))
        .collect()
}

pub(crate) struct QuadIndices {
    buffer: wgpu::Buffer,
    quads: u32,
    max_quads: u32,
}

impl QuadIndices {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let max_quads = (device.limits().max_buffer_size / QUAD_INDEX_BYTES)
            .min(u64::from(u32::MAX / 4)) as u32;
        let quads = INITIAL_QUADS.min(max_quads);
        QuadIndices {
            buffer: Self::create(device, quads),
            quads,
            max_quads,
        }
    }

    fn create(device: &wgpu::Device, quads: u32) -> wgpu::Buffer {
        let bytes: Vec<u8> = quad_indices(quads)
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect();
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad indices"),
            contents: &bytes,
            usage: wgpu::BufferUsages::INDEX,
        })
    }

    pub(crate) fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub(crate) fn bytes(&self) -> u64 {
        self.buffer.size()
    }

    pub(crate) fn ensure(&mut self, device: &wgpu::Device, quads: u32) -> Result<(), String> {
        if quads <= self.quads {
            return Ok(());
        }
        let bytes = grown_capacity(
            self.bytes(),
            u64::from(quads) * QUAD_INDEX_BYTES,
            u64::from(self.max_quads) * QUAD_INDEX_BYTES,
            QUAD_INDEX_BYTES,
        )
        .ok_or("quad index buffer is full")?;
        self.quads = (bytes / QUAD_INDEX_BYTES) as u32;
        self.buffer = Self::create(device, self.quads);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
