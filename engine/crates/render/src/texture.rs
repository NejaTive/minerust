use wgpu::util::DeviceExt;
use world::tile::{LAYER_COUNT, TILE_SIZE, Tile, tile_pixels};

pub const MIP_LEVELS: u32 = TILE_SIZE.ilog2() + 1;

pub fn generate() -> Vec<u8> {
    let mut data = Vec::new();
    for layer in 0..LAYER_COUNT {
        let mut level = match Tile::ALL.get(layer as usize) {
            Some(&tile) => tile_pixels(tile),
            None => vec![0; (TILE_SIZE * TILE_SIZE * 4) as usize],
        };
        let mut size = TILE_SIZE;
        data.extend_from_slice(&level);
        let cutout = Tile::ALL.get(layer as usize).is_some_and(|t| t.is_cutout());
        while size > 1 {
            level = downsample(&level, size, cutout);
            size /= 2;
            data.extend_from_slice(&level);
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

pub(crate) fn create_block_textures(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> wgpu::TextureView {
    let texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("blocks"),
            size: wgpu::Extent3d {
                width: TILE_SIZE,
                height: TILE_SIZE,
                depth_or_array_layers: LAYER_COUNT,
            },
            mip_level_count: MIP_LEVELS,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &generate(),
    );
    texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests;
