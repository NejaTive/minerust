use wgpu::util::DeviceExt;
use world::tile::Tile;

use world::textures::{LAYERS_PER_ARRAY, TextureSet};

pub fn mip_levels(size: u32) -> u32 {
    size.ilog2() + 1
}

pub fn generate(set: &TextureSet) -> Vec<u8> {
    let size = set.size();
    let mut data = Vec::new();
    for tile in Tile::ALL {
        let texture = set.tile(tile);
        for (index, frame) in texture.layers.chunks_exact(set.frame_bytes()).enumerate() {
            let cutout = tile.is_cutout() || index >= texture.frames as usize;
            let mut level = frame.to_vec();
            let mut size = size;
            data.extend_from_slice(&level);
            while size > 1 {
                level = downsample(&level, size, cutout);
                size /= 2;
                data.extend_from_slice(&level);
            }
        }
    }
    data
}

fn downsample(level: &[u8], size: u32, cutout: bool) -> Vec<u8> {
    let half = size / 2;
    let mut out = Vec::with_capacity((half * half * 4) as usize);
    for y in 0..half {
        for x in 0..half {
            let texels = [(0, 0), (1, 0), (0, 1), (1, 1)]
                .map(|(dx, dy)| (((2 * y + dy) * size + 2 * x + dx) * 4) as usize);
            if !cutout {
                for channel in 0..3 {
                    let sum: f32 = texels
                        .iter()
                        .map(|&i| srgb_to_linear(level[i + channel]))
                        .sum();
                    out.push(linear_to_srgb(sum / 4.0));
                }
                let alpha: u32 = texels.iter().map(|&i| u32::from(level[i + 3])).sum();
                out.push(((alpha + 2) / 4) as u8);
                continue;
            }
            let opaque: Vec<usize> = texels
                .iter()
                .copied()
                .filter(|&i| level[i + 3] >= 128)
                .collect();
            let colored = if opaque.is_empty() {
                &texels[..]
            } else {
                &opaque[..]
            };
            for channel in 0..3 {
                let sum: f32 = colored
                    .iter()
                    .map(|&i| srgb_to_linear(level[i + channel]))
                    .sum();
                out.push(linear_to_srgb(sum / colored.len() as f32));
            }
            out.push(if opaque.len() >= 2 { 255 } else { 0 });
        }
    }
    out
}

fn srgb_to_linear(c: u8) -> f32 {
    let c = f32::from(c) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> u8 {
    let c = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (c * 255.0).round() as u8
}

pub(crate) fn create_animation_table(device: &wgpu::Device, set: &TextureSet) -> wgpu::Buffer {
    let bytes: Vec<u8> = set
        .animation_table()
        .into_iter()
        .flatten()
        .flatten()
        .flat_map(f32::to_le_bytes)
        .collect();
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("animation"),
        contents: &bytes,
        usage: wgpu::BufferUsages::UNIFORM,
    })
}

pub(crate) fn layer_bytes(size: u32) -> usize {
    (0..mip_levels(size))
        .map(|level| ((size >> level).pow(2) * 4) as usize)
        .sum()
}

/// Второй массив начинается со слоя `LAYERS_PER_ARRAY`: лимит WebGPU по умолчанию — 256 слоёв
/// на текстуру; шейдер выбирает массив по номеру слоя (`sample_layer`).
pub(crate) fn split_layers(set: &TextureSet) -> [(u32, Vec<u8>); 2] {
    let mut data = generate(set);
    let first = (set.layer_count() as usize).min(LAYERS_PER_ARRAY);
    let rest = data.split_off(first * layer_bytes(set.size()));
    [
        (first as u32, data),
        (set.layer_count() - first as u32, rest),
    ]
}

pub(crate) fn create_block_textures(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    set: &TextureSet,
) -> [wgpu::TextureView; 2] {
    split_layers(set).map(|(layers, data)| {
        let layers = layers.max(1);
        let mut data = data;
        data.resize(layers as usize * layer_bytes(set.size()), 0);
        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("blocks"),
                size: wgpu::Extent3d {
                    width: set.size(),
                    height: set.size(),
                    depth_or_array_layers: layers,
                },
                mip_level_count: mip_levels(set.size()),
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &data,
        );
        texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        })
    })
}

#[cfg(test)]
mod tests;
