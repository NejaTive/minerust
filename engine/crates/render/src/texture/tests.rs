use super::*;

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
    let green = [58, 118, 38, 255];
    let hole = [0, 0, 0, 0];
    let level = [green, hole, hole, green].concat();
    assert_eq!(downsample(&level, 2, true), green);
    let level = [green, hole, hole, hole].concat();
    assert_eq!(downsample(&level, 2, true)[3], 0);
}

#[test]
fn translucent_downsample_averages_alpha() {
    let level = [[52, 98, 196, 180], [52, 98, 196, 100]].repeat(2).concat();
    assert_eq!(downsample(&level, 2, false), [52, 98, 196, 140]);
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
