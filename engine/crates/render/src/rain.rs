pub(crate) const RAIN_GRID: u32 = 32;
pub(crate) const RAIN_INSTANCES: u32 = RAIN_GRID * RAIN_GRID;
pub(crate) const TILE_WIDTH: u32 = 32;
pub(crate) const TILE_HEIGHT: u32 = 64;

const STREAK_SEED: u32 = 0x7261_696e;
const STREAK_LENGTH: std::ops::RangeInclusive<u32> = 3..=12;
const STREAKS: usize = 4;
const HEAD: [u8; 3] = [214, 232, 255];
const TAIL: [u8; 3] = [96, 136, 206];
const HEAD_ALPHA: f32 = 170.0;
const TAIL_ALPHA: f32 = 80.0;
pub(crate) const RAIN_VERTICES: u32 = 6;

const SNOW_SEED: u32 = 0x736e_6f77;
const SNOWFLAKES: usize = 28;
const SNOW_ALPHA: u8 = 210;

pub(crate) fn create_rain_cover_texture(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("rain cover"),
        size: wgpu::Extent3d {
            width: RAIN_GRID,
            height: RAIN_GRID,
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

fn next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state >> 8
}

pub(crate) fn rain_tile_pixels() -> Vec<u8> {
    let mut pixels = vec![0; (TILE_WIDTH * TILE_HEIGHT * 4) as usize];
    let mut state = STREAK_SEED;
    let mut used: Vec<u32> = Vec::new();
    while used.len() < STREAKS {
        let x = next(&mut state) % TILE_WIDTH;
        if used.iter().any(|&u| u.abs_diff(x) < 4) {
            continue;
        }
        used.push(x);
        let span = STREAK_LENGTH.end() - STREAK_LENGTH.start() + 1;
        let length = STREAK_LENGTH.start() + next(&mut state) % span;
        let start = next(&mut state) % (TILE_HEIGHT - length);
        let wide = next(&mut state).is_multiple_of(4) && x + 1 < TILE_WIDTH;
        for row in 0..length {
            let t = row as f32 / (length - 1) as f32;
            for column in x..=x + u32::from(wide) {
                let at = (((start + row) * TILE_WIDTH + column) * 4) as usize;
                for c in 0..3 {
                    let (head, tail) = (f32::from(HEAD[c]), f32::from(TAIL[c]));
                    pixels[at + c] = (head + (tail - head) * t) as u8;
                }
                pixels[at + 3] = (HEAD_ALPHA + (TAIL_ALPHA - HEAD_ALPHA) * t) as u8;
            }
        }
    }
    pixels
}

pub(crate) fn snow_tile_pixels() -> Vec<u8> {
    let mut pixels = vec![0; (TILE_WIDTH * TILE_HEIGHT * 4) as usize];
    let mut state = SNOW_SEED;
    let mut placed = 0;
    while placed < SNOWFLAKES {
        let x = next(&mut state) % (TILE_WIDTH - 1);
        let y = next(&mut state) % (TILE_HEIGHT - 1);
        let size = 1 + next(&mut state) % 2;
        let free = (y.saturating_sub(1)..=y + size).all(|row| {
            (x.saturating_sub(1)..=x + size).all(|column| {
                row >= TILE_HEIGHT
                    || column >= TILE_WIDTH
                    || pixels[((row * TILE_WIDTH + column) * 4 + 3) as usize] == 0
            })
        });
        if !free {
            continue;
        }
        for row in y..y + size {
            for column in x..x + size {
                let at = ((row * TILE_WIDTH + column) * 4) as usize;
                pixels[at..at + 4].copy_from_slice(&[255, 255, 255, SNOW_ALPHA]);
            }
        }
        placed += 1;
    }
    pixels
}

pub(crate) fn create_rain_tile_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> wgpu::TextureView {
    use wgpu::util::DeviceExt;
    device
        .create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("rain tile"),
                size: wgpu::Extent3d {
                    width: TILE_WIDTH,
                    height: TILE_HEIGHT * 2,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &[rain_tile_pixels(), snow_tile_pixels()].concat(),
        )
        .create_view(&wgpu::TextureViewDescriptor::default())
}

pub(crate) fn upload_rain_cover(queue: &wgpu::Queue, texture: &wgpu::Texture, heights: &[u8]) {
    queue.write_texture(
        texture.as_image_copy(),
        heights,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(RAIN_GRID),
            rows_per_image: Some(RAIN_GRID),
        },
        texture.size(),
    );
}

#[cfg(test)]
mod tests;
