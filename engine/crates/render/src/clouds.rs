use glam::Vec3;
use world::clouds::{CLOUD_MAP, CloudMask};

pub(crate) const CLOUD_RADIUS_CELLS: u32 = 32;
pub(crate) const CLOUD_GRID: u32 = 2 * CLOUD_RADIUS_CELLS;
pub(crate) const CLOUD_INSTANCES: u32 = CLOUD_GRID * CLOUD_GRID;
pub(crate) const CLOUD_VERTICES: u32 = 36;

const DAY_CLOUD: Vec3 = Vec3::new(1.0, 1.0, 1.0);
const DUSK_CLOUD: Vec3 = Vec3::new(1.0, 0.6, 0.42);
const NIGHT_CLOUD: Vec3 = Vec3::new(0.1, 0.11, 0.16);
const RAIN_CLOUD: Vec3 = Vec3::new(0.38, 0.4, 0.45);
const FLASH_CLOUD: Vec3 = Vec3::new(0.9, 0.93, 1.0);
const DUSK_STRENGTH: f32 = 0.8;

pub(crate) fn cloud_color(night: f32, warm: f32, rain: f32, flash: f32) -> Vec3 {
    let dull = DAY_CLOUD
        .lerp(NIGHT_CLOUD, night)
        .lerp(DUSK_CLOUD, warm * DUSK_STRENGTH);
    dull.lerp(RAIN_CLOUD * (1.0 - 0.7 * night), rain)
        .lerp(FLASH_CLOUD, flash * 0.8)
}

pub(crate) fn create_cloud_texture(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("clouds"),
        size: wgpu::Extent3d {
            width: CLOUD_MAP as u32,
            height: CLOUD_MAP as u32,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Uint,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

pub(crate) fn upload_cloud_mask(queue: &wgpu::Queue, texture: &wgpu::Texture, mask: &CloudMask) {
    queue.write_texture(
        texture.as_image_copy(),
        mask.cells(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(CLOUD_MAP as u32),
            rows_per_image: Some(CLOUD_MAP as u32),
        },
        texture.size(),
    );
}

#[cfg(test)]
mod tests;
