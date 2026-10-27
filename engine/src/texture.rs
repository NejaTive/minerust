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
    Sand,
    LogSide,
    LogTop,
    Leaves,
    Water,
}

impl Tile {
    pub const ALL: [Tile; 9] = [
        Tile::GrassTop,
        Tile::GrassSide,
        Tile::Dirt,
        Tile::Stone,
        Tile::Sand,
        Tile::LogSide,
        Tile::LogTop,
        Tile::Leaves,
        Tile::Water,
    ];

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
const SAND: [f32; 3] = [219.0, 205.0, 160.0];
const BARK: [f32; 3] = [92.0, 68.0, 40.0];
/// Годовые кольца на торце бревна: светлые и тёмные через одно.
const RINGS_LIGHT: [f32; 3] = [182.0, 148.0, 94.0];
const RINGS_DARK: [f32; 3] = [150.0, 118.0, 72.0];
const LEAVES: [f32; 3] = [58.0, 118.0, 38.0];
const WATER: [f32; 3] = [52.0, 98.0, 196.0];
/// Непрозрачность воды: дно на глубине пары блоков ещё видно, а поверхность читается.
const WATER_ALPHA: u8 = 180;
/// Доля дырок в листве: примерно как в Minecraft, крона ещё читается как масса.
const LEAF_HOLES: f32 = 0.3;
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
            Some(&tile) => tile_pixels(tile),
            None => vec![0; (TILE_SIZE * TILE_SIZE * 4) as usize],
        };
        let mut size = TILE_SIZE;
        data.extend_from_slice(&level);
        let cutout = Tile::ALL.get(layer as usize) == Some(&Tile::Leaves);
        while size > 1 {
            level = downsample(&level, size, cutout);
            size /= 2;
            data.extend_from_slice(&level);
        }
    }
    data
}

/// Пиксели тайла 16×16 RGBA (sRGB, строки сверху вниз) — те же, что базовый уровень на GPU.
pub fn tile_pixels(tile: Tile) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((TILE_SIZE * TILE_SIZE * 4) as usize);
    for y in 0..TILE_SIZE {
        for x in 0..TILE_SIZE {
            pixels.extend(pixel(tile, x, y).map(|c| c.round() as u8));
            // У дырок цвет тот же, что у листвы, а не чёрный: на смешении мип-уровней
            // он попадает в результат, и чёрный дал бы тёмную кайму.
            pixels.push(match tile {
                Tile::Water => WATER_ALPHA,
                _ if is_hole(tile, x, y) => 0,
                _ => 255,
            });
        }
    }
    pixels
}

/// Уровень вдвое меньше: каждый квадрат 2×2 → один пиксель. Цвет усредняется в
/// линейном пространстве — среднее sRGB-байтов темнее настоящего, и вдали тайлы бы
/// темнели.
///
/// Без `cutout` (вода) альфа — простое среднее: она и так полупрозрачная и
/// смешивается пайплайном, порога нет.
///
/// С `cutout` (листва) цвет берётся только по непрозрачным пикселям, чтобы цвет дырок
/// не просачивался, а альфа остаётся двоичной: 255, если непрозрачны хотя бы 2 из 4. Шейдер отбрасывает
/// пиксели с альфой < 0.5, и при простом среднем покрытие таяло бы с каждым уровнем —
/// кроны вдали «растворялись» бы. Большинство при 70% непрозрачных даёт ~92% на
/// следующем уровне: вдали листва плотнее, а не прозрачнее. Масштабирование альфы под
/// покрытие (как для травы с мягкой альфой) здесь лишнее: на уровнях 2×2 и 1×1 долю
/// 70% всё равно не выразить, а двоичная альфа не зависит от того, где GPU смешает уровни.
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
        Tile::Sand => SAND,
        Tile::Leaves => LEAVES,
        Tile::Water => WATER,
        // Продольные полосы коры: яркость по столбцу пикселей, заметнее пиксельного шума,
        // иначе вблизи кора неотличима от земли.
        Tile::LogSide => BARK.map(|c| c * (0.65 + 0.55 * noise(tile, x, TILE_SIZE))),
        Tile::LogTop => {
            // Квадратные кольца от центра; крайний ряд — кора.
            let ring = [x, y]
                .map(|c| (c as f32 - (TILE_SIZE - 1) as f32 / 2.0).abs())
                .into_iter()
                .fold(0.0, f32::max) as u32;
            if ring == TILE_SIZE / 2 - 1 {
                BARK
            } else if ring.is_multiple_of(2) {
                RINGS_LIGHT
            } else {
                RINGS_DARK
            }
        }
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

/// Прозрачный ли пиксель: дырки есть только у листвы.
fn is_hole(tile: Tile, x: u32, y: u32) -> bool {
    // Другой участок хеша, чем у яркости, чтобы дырки не совпадали с тёмными пикселями.
    tile == Tile::Leaves && noise(tile, x + TILE_SIZE, y) < LEAF_HOLES
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

    /// Мип-уровень `level` тайла: RGBA-пиксели стороной `TILE_SIZE >> level`.
    fn mip(data: &[u8], tile: Tile, level: u32) -> &[u8] {
        let start = tile.layer() as usize * layer_bytes()
            + (0..level)
                .map(|l| ((TILE_SIZE >> l).pow(2) * 4) as usize)
                .sum::<usize>();
        &data[start..start + ((TILE_SIZE >> level).pow(2) * 4) as usize]
    }

    fn coverage(pixels: &[u8]) -> f32 {
        let opaque = pixels.chunks(4).filter(|p| p[3] == 255).count();
        opaque as f32 / (pixels.len() / 4) as f32
    }

    #[test]
    fn generates_every_layer_with_all_mips() {
        let data = generate();
        assert_eq!(MIP_LEVELS, 5);
        // 16² + 8² + 4² + 2² + 1² = 341 пиксель на слой.
        assert_eq!(layer_bytes(), 341 * 4);
        assert_eq!(data.len(), LAYER_COUNT as usize * layer_bytes());
        for tile in Tile::ALL {
            let layer = &data[tile.layer() as usize * layer_bytes()..][..layer_bytes()];
            if tile == Tile::Water {
                assert!(layer.chunks(4).all(|p| p[3] == WATER_ALPHA), "{tile:?}");
                continue;
            }
            assert!(
                layer.chunks(4).all(|p| p[3] == 0 || p[3] == 255),
                "{tile:?}"
            );
            if tile != Tile::Leaves {
                assert!(layer.chunks(4).all(|p| p[3] == 255), "{tile:?}");
            }
        }
        assert_eq!(generate(), data, "generation must be deterministic");
    }

    #[test]
    fn tile_pixels_match_gpu_base_level() {
        let data = generate();
        for tile in Tile::ALL {
            let pixels = tile_pixels(tile);
            assert_eq!(
                pixels.len(),
                (TILE_SIZE * TILE_SIZE * 4) as usize,
                "{tile:?}"
            );
            assert_eq!(pixels, mip(&data, tile, 0), "{tile:?}");
        }
    }

    #[test]
    fn uniform_tile_keeps_color_down_to_1x1() {
        let color = [93, 155, 58, 255];
        let mut level = color.repeat((TILE_SIZE * TILE_SIZE) as usize);
        let mut size = TILE_SIZE;
        while size > 1 {
            level = downsample(&level, size, true);
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
        let out = downsample(&level, 2, true);
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

    #[test]
    fn leaves_keep_coverage_in_mips() {
        let data = generate();
        let base = coverage(mip(&data, Tile::Leaves, 0));
        assert!((0.55..0.85).contains(&base), "base coverage {base}");
        for level in 1..MIP_LEVELS {
            let c = coverage(mip(&data, Tile::Leaves, level));
            assert!(c >= base, "level {level}: {c} < {base}");
        }
    }

    #[test]
    fn leaves_mips_stay_leaf_colored() {
        // Диапазон яркости базового уровня; среднее в линейном пространстве из него не выходит.
        let data = generate();
        for level in 1..MIP_LEVELS {
            for p in mip(&data, Tile::Leaves, level).chunks(4) {
                for (c, base) in p[..3].iter().zip(LEAVES) {
                    let c = f32::from(*c);
                    assert!(
                        (base * 0.82 - 1.0..=base * 1.18 + 1.0).contains(&c),
                        "level {level}: {p:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn downsample_ignores_color_of_holes() {
        // Две зелёные непрозрачные и две чёрные дырки: цвет зелёный, без потемнения.
        let green = [58, 118, 38, 255];
        let hole = [0, 0, 0, 0];
        let level = [green, hole, hole, green].concat();
        assert_eq!(downsample(&level, 2, true), green);
        // Одна непрозрачная из четырёх — дырка.
        let level = [green, hole, hole, hole].concat();
        assert_eq!(downsample(&level, 2, true)[3], 0);
    }

    #[test]
    fn translucent_downsample_averages_alpha() {
        let level = [[52, 98, 196, 180], [52, 98, 196, 100]].repeat(2).concat();
        assert_eq!(downsample(&level, 2, false), [52, 98, 196, 140]);
        // Листва с той же альфой стала бы непрозрачной.
        assert_eq!(downsample(&level, 2, true)[3], 255);
    }

    #[test]
    fn layer_count_avoids_gl_guessing() {
        assert!(LAYER_COUNT != 1 && !LAYER_COUNT.is_multiple_of(6));
        assert!(LAYER_COUNT as usize >= Tile::ALL.len());
    }
}
