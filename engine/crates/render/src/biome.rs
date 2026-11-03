use world::pos::ColumnPos;
use world::section::SECTION_SIZE;

pub const BIOME_MAP_SIZE: u32 = 1024;
pub const PALETTE_BYTES: u64 = 8 * 16;

pub(crate) fn create_biome_map(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("biome weights"),
        size: wgpu::Extent3d {
            width: BIOME_MAP_SIZE,
            height: BIOME_MAP_SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

pub(crate) fn column_origin(key: ColumnPos) -> (u32, u32) {
    let wrap = |v: i32| (v * SECTION_SIZE).rem_euclid(BIOME_MAP_SIZE as i32) as u32;
    (wrap(key.x()), wrap(key.z()))
}

pub(crate) fn upload_biome_column(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    key: ColumnPos,
    weights: &[u8],
) {
    let (x, z) = column_origin(key);
    let side = SECTION_SIZE as u32;
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x, y: z, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        weights,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(side * 4),
            rows_per_image: Some(side),
        },
        wgpu::Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: 1,
        },
    );
}

#[cfg(test)]
mod tests;
