/// Сторона тайла в пикселях.
pub const TILE_SIZE: u32 = 16;

/// Тайл текстуры блока; значение — номер слоя в массиве текстур.
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    GrassTop,
    GrassSide,
    Dirt,
    Stone,
}

impl Tile {
    pub const ALL: [Tile; 4] = [Tile::GrassTop, Tile::GrassSide, Tile::Dirt, Tile::Stone];

    pub fn layer(self) -> u32 {
        self as u32
    }
}

/// Сколько слоёв выделить на GPU: на один больше, если тайлов 1 или кратно 6.
/// GL-бэкенд wgpu угадывает тип текстуры по числу слоёв (`get_info_from_desc` в wgpu-hal
/// 30, issues gfx-rs/wgpu#1574, #1614): 1 слой — `TEXTURE_2D`, квадратная с кратным 6 —
/// cube map. Оба не совпадут с `texture_2d_array` в шейдере, и на WebGL2 будет ошибка.
pub const LAYER_COUNT: u32 = {
    let tiles = Tile::ALL.len() as u32;
    if tiles == 1 || tiles.is_multiple_of(6) {
        tiles + 1
    } else {
        tiles
    }
};

// Базовые цвета в sRGB: текстура `Rgba8UnormSrgb`, в линейный цвет переводит GPU.
const GRASS: [f32; 3] = [93.0, 155.0, 58.0];
const DIRT: [f32; 3] = [134.0, 94.0, 60.0];
const STONE: [f32; 3] = [128.0, 128.0, 128.0];
/// Глубина зелёной каймы на боку травы, пикселей сверху.
const GRASS_FRINGE: u32 = 3;

/// Мип-уровней на слой: 16 → 8 → 4 → 2 → 1.
pub const MIP_LEVELS: u32 = TILE_SIZE.ilog2() + 1;

/// Данные всех `LAYER_COUNT` слоёв в порядке `TextureDataOrder::LayerMajor`: для каждого
/// слоя все его мип-уровни подряд, от 16×16 до 1×1. Пиксели RGBA8 sRGB, строки сверху
/// вниз (v = 0 — верх тайла). Лишний выравнивающий слой, если есть, прозрачный.
pub fn generate() -> Vec<u8> {
    let mut data = Vec::new();
    for layer in 0..LAYER_COUNT {
        let mut level = match Tile::ALL.get(layer as usize) {
            Some(&tile) => base_level(tile),
            None => vec![0; (TILE_SIZE * TILE_SIZE * 4) as usize],
        };
        let mut size = TILE_SIZE;
        data.extend_from_slice(&level);
        while size > 1 {
            level = downsample(&level, size);
            size /= 2;
            data.extend_from_slice(&level);
        }
    }
    data
}

fn base_level(tile: Tile) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((TILE_SIZE * TILE_SIZE * 4) as usize);
    for y in 0..TILE_SIZE {
        for x in 0..TILE_SIZE {
            pixels.extend(pixel(tile, x, y).map(|c| c.round() as u8));
            pixels.push(255);
        }
    }
    pixels
}

/// Уровень вдвое меньше: среднее каждого квадрата 2×2. Цвет усредняется в линейном
/// пространстве — среднее sRGB-байтов темнее настоящего, и вдали тайлы бы темнели.
fn downsample(level: &[u8], size: u32) -> Vec<u8> {
    let half = size / 2;
    let mut out = Vec::with_capacity((half * half * 4) as usize);
    for y in 0..half {
        for x in 0..half {
            let texels = [(0, 0), (1, 0), (0, 1), (1, 1)]
                .map(|(dx, dy)| (((2 * y + dy) * size + 2 * x + dx) * 4) as usize);
            for channel in 0..3 {
                let sum: f32 = texels
                    .iter()
                    .map(|&i| srgb_to_linear(level[i + channel]))
                    .sum();
                out.push(linear_to_srgb(sum / 4.0));
            }
            let alpha: u32 = texels.iter().map(|&i| u32::from(level[i + 3])).sum();
            out.push(((alpha + 2) / 4) as u8);
        }
    }
    out
}

/// Формулы sRGB (IEC 61966-2-1), те же, что применяет GPU для `*UnormSrgb`.
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

fn pixel(tile: Tile, x: u32, y: u32) -> [f32; 3] {
    let base = match tile {
        Tile::GrassTop => GRASS,
        Tile::Dirt => DIRT,
        Tile::Stone => STONE,
        Tile::GrassSide => {
            // Неровный край: в каждом столбце кайма на пиксель длиннее с вероятностью 1/2.
            let fringe = GRASS_FRINGE + u32::from(noise(tile, x, TILE_SIZE) > 0.5);
            if y < fringe { GRASS } else { DIRT }
        }
    };
    // ±18% яркости по пикселям — пока вместо нарисованной текстуры.
    let brightness = 0.82 + 0.36 * noise(tile, x, y);
    base.map(|c| (c * brightness).min(255.0))
}

/// Детерминированный «шум» 0..=1 по пикселю: целочисленный хеш, без зависимостей.
fn noise(tile: Tile, x: u32, y: u32) -> f32 {
    let mut h = x.wrapping_mul(0x8da6_b343)
        ^ y.wrapping_mul(0xd816_3841)
        ^ tile.layer().wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h & 0xffff) as f32 / 65535.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer_bytes() -> usize {
        (0..MIP_LEVELS)
            .map(|level| ((TILE_SIZE >> level).pow(2) * 4) as usize)
            .sum()
    }

    fn rgb(data: &[u8], tile: Tile, x: u32, y: u32) -> [u8; 3] {
        let i = tile.layer() as usize * layer_bytes() + ((y * TILE_SIZE + x) * 4) as usize;
        [data[i], data[i + 1], data[i + 2]]
    }

    #[test]
    fn generates_every_layer_with_all_mips() {
        let data = generate();
        assert_eq!(MIP_LEVELS, 5);
        // 16² + 8² + 4² + 2² + 1² = 341 пиксель на слой.
        assert_eq!(layer_bytes(), 341 * 4);
        assert_eq!(data.len(), LAYER_COUNT as usize * layer_bytes());
        let used = &data[..Tile::ALL.len() * layer_bytes()];
        assert!(used.chunks(4).all(|p| p[3] == 255));
        assert_eq!(generate(), data, "generation must be deterministic");
    }

    #[test]
    fn uniform_tile_keeps_color_down_to_1x1() {
        let color = [93, 155, 58, 255];
        let mut level = color.repeat((TILE_SIZE * TILE_SIZE) as usize);
        let mut size = TILE_SIZE;
        while size > 1 {
            level = downsample(&level, size);
            size /= 2;
        }
        assert_eq!(level, color);
    }

    #[test]
    fn downsample_averages_in_linear_space() {
        // Шахматка чёрного и белого: в линейном пространстве среднее 0.5 → sRGB ≈ 188,
        // а не 128, как дало бы среднее байтов.
        let black = [0, 0, 0, 255];
        let white = [255, 255, 255, 255];
        let level = [white, black, black, white].concat();
        let out = downsample(&level, 2);
        assert!((187..=188).contains(&out[0]), "got {}", out[0]);
        assert_eq!(out[3], 255);
    }

    #[test]
    fn grass_side_is_green_on_top_and_dirt_below() {
        let pixels = generate();
        for x in 0..TILE_SIZE {
            let [r, g, _] = rgb(&pixels, Tile::GrassSide, x, 0);
            assert!(g > r, "top row should be green at x = {x}");
            let [r, g, _] = rgb(&pixels, Tile::GrassSide, x, TILE_SIZE - 1);
            assert!(r > g, "bottom row should be dirt at x = {x}");
        }
    }
}
