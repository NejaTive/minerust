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
    Glowstone,
    TallGrass,
    Poppy,
    Dandelion,
    Torch,
}

impl Tile {
    pub const ALL: [Tile; 14] = [
        Tile::GrassTop,
        Tile::GrassSide,
        Tile::Dirt,
        Tile::Stone,
        Tile::Sand,
        Tile::LogSide,
        Tile::LogTop,
        Tile::Leaves,
        Tile::Water,
        Tile::Glowstone,
        Tile::TallGrass,
        Tile::Poppy,
        Tile::Dandelion,
        Tile::Torch,
    ];

    pub fn layer(self) -> u32 {
        self as u32
    }

    /// Тайл с дырками (двоичная альфа, мипы по большинству): листва, растения, факел.
    pub fn is_cutout(self) -> bool {
        matches!(
            self,
            Tile::Leaves | Tile::TallGrass | Tile::Poppy | Tile::Dandelion | Tile::Torch
        )
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
const GRASS: [f32; 3] = [108.0, 168.0, 52.0];
const DIRT: [f32; 3] = [134.0, 94.0, 60.0];
const STONE: [f32; 3] = [128.0, 128.0, 128.0];
const SAND: [f32; 3] = [219.0, 205.0, 160.0];
const BARK: [f32; 3] = [92.0, 68.0, 40.0];
/// Годовые кольца на торце бревна: светлые и тёмные через одно.
const RINGS_LIGHT: [f32; 3] = [182.0, 148.0, 94.0];
const RINGS_DARK: [f32; 3] = [150.0, 118.0, 72.0];
const LEAVES: [f32; 3] = [52.0, 100.0, 30.0];
const WATER: [f32; 3] = [52.0, 98.0, 196.0];
/// Светокамень: тёплый жёлтый фон, светлые прожилки и тёмные впадины крупными
/// пятнами 2×2 — на мипах 4×4 и 2×2 рисунок ещё читается.
const GLOW_BASE: [f32; 3] = [226.0, 170.0, 72.0];
const GLOW_BRIGHT: [f32; 3] = [255.0, 236.0, 150.0];
const GLOW_DARK: [f32; 3] = [170.0, 108.0, 40.0];
/// Непрозрачность воды: дно на глубине пары блоков ещё видно, а поверхность читается.
/// Высокая трава: тёмный зелёный у корня и светлый желтоватый на кончиках (как в
/// Minecraft). Светлее земной травы вверху, чтобы пучок отличался от луга, и темнее
/// внизу, чтобы стоял на земле, а не парил.
const TALL_GRASS_ROOT: [f32; 3] = [84.0, 124.0, 44.0];
const TALL_GRASS_TIP: [f32; 3] = [150.0, 196.0, 80.0];
const STEM: [f32; 3] = [60.0, 130.0, 40.0];
const POPPY_PETAL: [f32; 3] = [214.0, 40.0, 36.0];
const POPPY_CORE: [f32; 3] = [40.0, 24.0, 24.0];
const DANDELION_PETAL: [f32; 3] = [250.0, 214.0, 40.0];
/// Древко факела и огонёк на его верхних двух пикселях.
const TORCH_STICK: [f32; 3] = [120.0, 84.0, 48.0];
const TORCH_FLAME: [f32; 3] = [255.0, 200.0, 70.0];
/// Столбик факела в тайле: столбцы 7..9, строки 6..16 (10 из 16, как 2×10×2 в мешере).
pub const TORCH_PIXELS: (u32, u32) = (7, 9);
pub const TORCH_TOP_ROW: u32 = 6;
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
        let cutout = Tile::ALL.get(layer as usize).is_some_and(|t| t.is_cutout());
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
        Tile::TallGrass | Tile::Poppy | Tile::Dandelion | Tile::Torch => {
            plant_pixel(tile, x, y).unwrap_or(STEM)
        }
        Tile::Glowstone => match noise(tile, x / 2, y / 2) {
            n if n > 0.7 => GLOW_BRIGHT,
            n if n < 0.3 => GLOW_DARK,
            _ => GLOW_BASE,
        },
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

/// Цвет пикселя растения или факела; `None` — дырка.
fn plant_pixel(tile: Tile, x: u32, y: u32) -> Option<[f32; 3]> {
    let in_rect =
        |x0: u32, x1: u32, y0: u32, y1: u32| (x0..x1).contains(&x) && (y0..y1).contains(&y);
    match tile {
        Tile::TallGrass => {
            // Пучок травинок: часть столбцов пуста, высота остальных разная.
            let present = noise(tile, x, TILE_SIZE + 1) > 0.2;
            let height = 6 + (10.0 * noise(tile, x, TILE_SIZE)) as u32;
            let tip = y as f32 / (TILE_SIZE - 1) as f32;
            // y растёт вниз: у корня (tip = 1) цвет тёмный.
            let color = std::array::from_fn(|i| {
                TALL_GRASS_TIP[i] + (TALL_GRASS_ROOT[i] - TALL_GRASS_TIP[i]) * tip
            });
            (present && y >= TILE_SIZE - height).then_some(color)
        }
        Tile::Poppy | Tile::Dandelion => {
            let (petal, core, head) = if tile == Tile::Poppy {
                (POPPY_PETAL, POPPY_CORE, (5, 11, 2, 7))
            } else {
                (DANDELION_PETAL, DANDELION_PETAL, (6, 10, 3, 7))
            };
            if in_rect(7, 9, 7, 16) || [(5, 11), (6, 10), (9, 10), (10, 11)].contains(&(x, y)) {
                Some(STEM)
            } else if in_rect(7, 9, 4, 6) && tile == Tile::Poppy {
                Some(core)
            } else if in_rect(head.0, head.1, head.2, head.3) {
                Some(petal)
            } else {
                None
            }
        }
        Tile::Torch => {
            let (x0, x1) = TORCH_PIXELS;
            if in_rect(x0, x1, TORCH_TOP_ROW, TORCH_TOP_ROW + 2) {
                Some(TORCH_FLAME)
            } else if in_rect(x0, x1, TORCH_TOP_ROW + 2, TILE_SIZE) {
                Some(TORCH_STICK)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Прозрачный ли пиксель: дырки есть у листвы, растений и факела.
fn is_hole(tile: Tile, x: u32, y: u32) -> bool {
    match tile {
        // Другой участок хеша, чем у яркости, чтобы дырки не совпадали с тёмными пикселями.
        Tile::Leaves => noise(tile, x + TILE_SIZE, y) < LEAF_HOLES,
        Tile::TallGrass | Tile::Poppy | Tile::Dandelion | Tile::Torch => {
            plant_pixel(tile, x, y).is_none()
        }
        _ => false,
    }
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
            if !tile.is_cutout() {
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

    #[test]
    fn layer_count_for_current_tiles() {
        // 14 тайлов: не 1 и не кратно 6, выравнивающий слой не нужен.
        assert_eq!(Tile::ALL.len(), 14);
        assert_eq!(LAYER_COUNT, 14);
    }

    #[test]
    fn plant_tiles_have_holes_and_binary_mips() {
        let data = generate();
        for tile in [Tile::TallGrass, Tile::Poppy, Tile::Dandelion, Tile::Torch] {
            for level in 0..MIP_LEVELS {
                let c = coverage(mip(&data, tile, level));
                if level == 0 {
                    assert!(c > 0.05 && c < 0.7, "{tile:?} coverage {c}");
                }
                assert!(
                    mip(&data, tile, level)
                        .chunks(4)
                        .all(|p| p[3] == 0 || p[3] == 255),
                    "{tile:?} level {level}"
                );
            }
        }
    }

    #[test]
    fn torch_flame_is_on_top_of_stick() {
        let px = tile_pixels(Tile::Torch);
        let at = |x: u32, y: u32| &px[((y * TILE_SIZE + x) * 4) as usize..][..4];
        assert_eq!(at(7, TORCH_TOP_ROW)[3], 255);
        assert_eq!(at(8, 15)[3], 255);
        assert_eq!(at(6, 10)[3], 0);
        assert_eq!(at(7, TORCH_TOP_ROW - 1)[3], 0);
        assert!(at(7, TORCH_TOP_ROW)[0] > at(7, 12)[0], "flame is brighter");
    }
}
