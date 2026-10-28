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

/// Сколько слоёв выделить на GPU: по слою на тайл.
pub const LAYER_COUNT: u32 = Tile::ALL.len() as u32;

/// Столбик факела в тайле: столбцы 7..9, строки 6..16 (10 из 16, как 2×10×2 в мешере).
/// Договорённость с `torch.png`: мешер строит по ней коробку и верх, картинка обязана
/// быть непрозрачной именно здесь (проверяет тест).
pub const TORCH_PIXELS: (u32, u32) = (7, 9);
pub const TORCH_TOP_ROW: u32 = 6;

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

impl Tile {
    /// Имя файла в `engine/assets/textures/`.
    fn file_name(self) -> String {
        let name = format!("{self:?}");
        let mut out = String::new();
        for (i, c) in name.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        }
        out
    }

    /// PNG тайла, вшитый в бинарник. Файлы лежат в `engine/assets/textures/<имя_в_snake_case>.png`;
    /// новый тайл — это PNG 16×16 плюс вариант в `Tile` и в `ALL`.
    pub fn png(self) -> &'static [u8] {
        macro_rules! files {
            ($($tile:ident => $file:literal),* $(,)?) => {
                match self {
                    $(Tile::$tile => include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"), "/assets/textures/", $file, ".png"
                    )),)*
                }
            };
        }
        files! {
            GrassTop => "grass_top", GrassSide => "grass_side", Dirt => "dirt",
            Stone => "stone", Sand => "sand", LogSide => "log_side", LogTop => "log_top",
            Leaves => "leaves", Water => "water", Glowstone => "glowstone",
            TallGrass => "tall_grass", Poppy => "poppy", Dandelion => "dandelion",
            Torch => "torch",
        }
    }
}

/// Пиксели тайла 16×16 RGBA (sRGB, строки сверху вниз) — те же, что базовый уровень на GPU.
/// Байты PNG берутся как есть, цветовые чанки (gAMA, sRGB, iCCP) игнорируются.
pub fn tile_pixels(tile: Tile) -> Vec<u8> {
    let name = tile.file_name();
    let mut decoder = png::Decoder::new(std::io::Cursor::new(tile.png()));
    // Палитра, grayscale и 16 бит, которые сохраняют редакторы, приводятся к RGBA8.
    decoder.set_transformations(
        png::Transformations::ALPHA | png::Transformations::EXPAND | png::Transformations::STRIP_16,
    );
    let mut reader = decoder
        .read_info()
        .unwrap_or_else(|e| panic!("{name}.png: {e}"));
    let mut buf = vec![0; reader.output_buffer_size().expect("png size overflow")];
    let info = reader
        .next_frame(&mut buf)
        .unwrap_or_else(|e| panic!("{name}.png: {e}"));
    assert!(
        info.width == TILE_SIZE && info.height == TILE_SIZE,
        "{name}.png: expected {TILE_SIZE}x{TILE_SIZE}, got {}x{}",
        info.width,
        info.height
    );
    assert!(
        info.color_type == png::ColorType::Rgba && info.bit_depth == png::BitDepth::Eight,
        "{name}.png: unexpected output format {:?}/{:?}",
        info.color_type,
        info.bit_depth
    );
    buf.truncate(info.buffer_size());
    buf
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Альфа во всех пикселях `water.png`: дно на глубине пары блоков ещё видно.
    const WATER_ALPHA: u8 = 180;

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
    fn leaves_mips_stay_within_base_color_range() {
        // Среднее в линейном пространстве по непрозрачным пикселям не выходит за диапазон
        // непрозрачных пикселей базового уровня.
        let data = generate();
        let base: Vec<&[u8]> = mip(&data, Tile::Leaves, 0)
            .chunks(4)
            .filter(|p| p[3] == 255)
            .collect();
        for channel in 0..3 {
            let lo = base.iter().map(|p| p[channel]).min().unwrap();
            let hi = base.iter().map(|p| p[channel]).max().unwrap();
            for level in 1..MIP_LEVELS {
                for p in mip(&data, Tile::Leaves, level).chunks(4) {
                    assert!(
                        (lo.saturating_sub(1)..=hi.saturating_add(1)).contains(&p[channel]),
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
    fn layer_count_for_current_tiles() {
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
    fn torch_png_is_opaque_exactly_in_torch_geometry() {
        let px = tile_pixels(Tile::Torch);
        for y in 0..TILE_SIZE {
            for x in 0..TILE_SIZE {
                let alpha = px[((y * TILE_SIZE + x) * 4 + 3) as usize];
                let inside = (TORCH_PIXELS.0..TORCH_PIXELS.1).contains(&x) && y >= TORCH_TOP_ROW;
                assert_eq!(alpha == 255, inside, "({x}, {y}) alpha {alpha}");
                assert!(alpha == 0 || alpha == 255);
            }
        }
    }

    #[test]
    fn every_tile_has_a_16x16_png_and_no_extra_files() {
        // tile_pixels паникует с именем файла, если размер или формат не подходят.
        for tile in Tile::ALL {
            assert_eq!(
                tile_pixels(tile).len(),
                (TILE_SIZE * TILE_SIZE * 4) as usize
            );
        }
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/textures");
        let mut files: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        files.sort();
        let mut expected: Vec<String> = Tile::ALL
            .iter()
            .map(|t| format!("{}.png", t.file_name()))
            .collect();
        expected.sort();
        assert_eq!(files, expected);
        assert_eq!(files.len(), Tile::ALL.len());
    }

    #[test]
    fn cutout_tiles_have_holes_and_opaque_pixels() {
        for tile in Tile::ALL {
            let px = tile_pixels(tile);
            let opaque = px.chunks(4).filter(|p| p[3] == 255).count();
            let clear = px.chunks(4).filter(|p| p[3] == 0).count();
            if tile.is_cutout() {
                assert!(
                    opaque > 0 && clear > 0,
                    "{tile:?}: opaque {opaque}, clear {clear}"
                );
            } else if tile == Tile::Water {
                assert!(px.chunks(4).all(|p| p[3] == WATER_ALPHA), "{tile:?}");
            } else {
                assert!(px.chunks(4).all(|p| p[3] == 255), "{tile:?}");
            }
        }
    }

    #[test]
    #[ignore]
    fn generate_timing() {
        let start = std::time::Instant::now();
        for _ in 0..100 {
            std::hint::black_box(generate());
        }
        println!("generate: {:?} per call", start.elapsed() / 100);
    }
}
