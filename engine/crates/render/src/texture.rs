use world::tile::{LAYER_COUNT, TILE_SIZE, Tile, tile_pixels};

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
    #[ignore]
    fn generate_timing() {
        let start = std::time::Instant::now();
        for _ in 0..100 {
            std::hint::black_box(generate());
        }
        println!("generate: {:?} per call", start.elapsed() / 100);
    }
}
