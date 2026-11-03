use super::*;
use world::textures::{TextureSet, TileTexture};

const TILE_SIZE: u32 = 32;
const MIP_LEVELS: u32 = 6;
const LAYER_COUNT: u32 = 97;

fn standard() -> TextureSet {
    TextureSet::standard()
}

fn generate_standard() -> Vec<u8> {
    generate(&standard())
}

fn tile_pixels(tile: Tile) -> Vec<u8> {
    standard().tile_pixels(tile)
}

fn tile_frames(tile: Tile) -> Vec<u8> {
    standard().tile(tile).layers.clone()
}

fn first_layer(tile: Tile) -> usize {
    standard().first_layer(tile) as usize
}

const WATER_ALPHA: u8 = 180;

fn layer_bytes() -> usize {
    (0..MIP_LEVELS)
        .map(|level| ((TILE_SIZE >> level).pow(2) * 4) as usize)
        .sum()
}

fn rgb(data: &[u8], tile: Tile, x: u32, y: u32) -> [u8; 3] {
    let i = first_layer(tile) * layer_bytes() + ((y * TILE_SIZE + x) * 4) as usize;
    [data[i], data[i + 1], data[i + 2]]
}

fn mip(data: &[u8], tile: Tile, level: u32) -> &[u8] {
    let start = first_layer(tile) * layer_bytes()
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
    let data = generate_standard();
    assert_eq!(mip_levels(TILE_SIZE), MIP_LEVELS);
    assert_eq!(layer_bytes(), 1365 * 4);
    assert_eq!(data.len(), LAYER_COUNT as usize * layer_bytes());
    for tile in Tile::ALL {
        let layer = &data[first_layer(tile) * layer_bytes()..][..layer_bytes()];
        if tile == Tile::Water {
            assert!(
                layer.chunks(4).all(|p| (1..=WATER_ALPHA).contains(&p[3])),
                "{tile:?}"
            );
            continue;
        }
        if tile == Tile::Ice {
            assert!(
                layer.chunks(4).all(|p| (100..255).contains(&p[3])),
                "{tile:?}"
            );
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
    assert_eq!(
        generate_standard(),
        data,
        "generation must be deterministic"
    );
}

#[test]
fn tile_pixels_match_gpu_base_level() {
    let data = generate_standard();
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
    let pixels = generate_standard();
    let mean = |y: u32| {
        let sum = |c: usize| {
            (0..TILE_SIZE)
                .map(|x| u32::from(rgb(&pixels, Tile::GrassSide, x, y)[c]))
                .sum::<u32>()
        };
        (sum(0), sum(1))
    };
    let (r, g) = mean(0);
    assert!(g > r, "top row should be greener than red");
    let (r, g) = mean(TILE_SIZE - 1);
    assert!(r > g, "bottom row should be dirt");
}

#[test]
fn leaves_keep_coverage_in_mips() {
    let data = generate_standard();
    let base = coverage(mip(&data, Tile::Leaves, 0));
    assert!((0.55..0.85).contains(&base), "base coverage {base}");
    for level in 1..MIP_LEVELS {
        let c = coverage(mip(&data, Tile::Leaves, level));
        assert!(c >= base, "level {level}: {c} < {base}");
    }
}

#[test]
fn leaves_mips_stay_within_base_color_range() {
    let data = generate_standard();
    let base: Vec<&[u8]> = mip(&data, Tile::Leaves, 0)
        .chunks(4)
        .filter(|p| p[3] == 255)
        .collect();
    for channel in 0..3 {
        let lo = base.iter().map(|p| p[channel]).min().unwrap();
        let hi = base.iter().map(|p| p[channel]).max().unwrap();
        for level in 1..MIP_LEVELS {
            for p in mip(&data, Tile::Leaves, level)
                .chunks(4)
                .filter(|p| p[3] == 255)
            {
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
    let data = generate_standard();
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
        std::hint::black_box(generate_standard());
    }
    println!("generate: {:?} per call", start.elapsed() / 100);
}

#[test]
fn every_frame_of_animated_tiles_gets_its_own_layer() {
    let data = generate_standard();
    let frame_bytes = (TILE_SIZE * TILE_SIZE * 4) as usize;
    for tile in [Tile::Water, Tile::Lava] {
        let frames = tile_frames(tile);
        assert_eq!(
            frames.len(),
            standard().tile(tile).frames as usize * frame_bytes
        );
        for (i, frame) in frames.chunks_exact(frame_bytes).enumerate() {
            let layer = first_layer(tile) + i;
            let start = layer * layer_bytes();
            assert_eq!(
                &data[start..start + frame_bytes],
                frame,
                "{tile:?} frame {i}"
            );
        }
        assert_ne!(
            &frames[..frame_bytes],
            &frames[frame_bytes..2 * frame_bytes]
        );
    }
}

#[test]
fn builtin_set_output_matches_golden() {
    let data = generate_standard();
    let hash = data.iter().fold(0xcbf29ce484222325_u64, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    });
    assert_eq!(data.len(), 529620);
    assert_eq!(format!("{hash:016x}"), "f7ee3183cba82a85");
}

#[test]
fn mip_levels_follow_tile_size() {
    assert_eq!(mip_levels(16), 5);
    assert_eq!(mip_levels(32), 6);
    assert_eq!(mip_levels(512), 10);
}

#[test]
fn generates_all_mips_for_32px_tiles() {
    let tiles = Tile::ALL
        .iter()
        .map(|_| TileTexture::still([10, 20, 30, 255].repeat(32 * 32)))
        .collect();
    let set = TextureSet::from_tiles(32, tiles, world::biome_color::BiomePalette::constants());
    let data = generate(&set);
    let layer_bytes: usize = (0..6).map(|l| ((32 >> l) * (32 >> l) * 4) as usize).sum();
    assert_eq!(layer_bytes, 1365 * 4);
    assert_eq!(data.len(), Tile::ALL.len() * layer_bytes);
    assert_eq!(&data[layer_bytes - 4..layer_bytes], [10, 20, 30, 255]);
}
