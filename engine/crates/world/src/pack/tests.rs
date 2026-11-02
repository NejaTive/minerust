use std::io::Write;

use super::*;

const PREFIX: &str = "assets/minecraft/textures/blocks/";

fn png(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer
        .write_image_data(&rgba.repeat((width * height) as usize))
        .unwrap();
    writer.finish().unwrap();
    out
}

fn zip(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in files {
        writer.start_file(*name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn block(name: &str) -> String {
    format!("{PREFIX}{name}")
}

#[test]
fn rejects_garbage_and_empty_packs() {
    assert!(load(b"not a zip").is_err());
    let empty = zip(&[("readme.txt", b"hi".to_vec())]);
    assert!(load(&empty).is_err());
    let no_textures = zip(&[(&block("unknown.png"), png(16, 16, [1, 2, 3, 255]))]);
    assert!(load(&no_textures).is_err());
}

#[test]
fn maps_1_12_names_and_falls_back_to_standard() {
    let bytes = zip(&[
        (&block("dirt.png"), png(32, 32, [10, 20, 30, 255])),
        (&block("log_oak.png"), png(32, 32, [40, 50, 60, 255])),
        (&block("tallgrass.png"), png(32, 32, [70, 80, 90, 255])),
        (&block("stone.png"), b"broken".to_vec()),
    ]);
    let pack = load(&bytes).unwrap();
    let set = &pack.textures;
    assert_eq!(set.size(), 32);
    assert_eq!(&set.tile_pixels(Tile::Dirt)[..4], [10, 20, 30, 255]);
    assert_eq!(&set.tile_pixels(Tile::LogSide)[..4], [40, 50, 60, 255]);
    assert_eq!(&set.tile_pixels(Tile::TallGrass)[..4], [70, 80, 90, 255]);
    let standard = TextureSet::standard();
    let stone = set.tile_pixels(Tile::Stone);
    assert_eq!(stone.len(), 32 * 32 * 4);
    assert_eq!(
        scale_nearest(&standard.tile_pixels(Tile::Stone), 16, 32),
        stone
    );
    assert_eq!(set.tile(Tile::Stone).tint, WHITE);
}

#[test]
fn layer_count_matches_frames() {
    let bytes = zip(&[(&block("dirt.png"), png(16, 16, [1, 1, 1, 255]))]);
    let set = load(&bytes).unwrap().textures;
    let total: usize = Tile::ALL.iter().map(|&t| set.tile(t).frames as usize).sum();
    assert_eq!(set.layer_count() as usize, total);
    for tile in Tile::ALL {
        let texture = set.tile(tile);
        assert_eq!(
            texture.layers.len(),
            texture.frames as usize * set.frame_bytes()
        );
    }
}

#[test]
fn tints_follow_vanilla_colors() {
    let bytes = zip(&[
        (&block("grass_top.png"), png(16, 16, [200, 200, 200, 255])),
        (
            &block("leaves_birch.png"),
            png(16, 16, [200, 200, 200, 255]),
        ),
        (&block("water_still.png"), png(16, 16, [200, 200, 200, 255])),
    ]);
    let set = load(&bytes).unwrap().textures;
    let hex = |c: [f32; 3]| c.map(|v| (v * 255.0).round() as u8);
    assert_eq!(hex(set.tile(Tile::GrassTop).tint), [0x79, 0xC0, 0x5A]);
    assert_eq!(hex(set.tile(Tile::TallGrass).tint), WHITE.map(|_| 255));
    assert_eq!(hex(set.tile(Tile::BirchLeaves).tint), [0x80, 0xA7, 0x55]);
    assert_eq!(hex(set.tile(Tile::Water).tint), [0x3F, 0x76, 0xE4]);
    assert_eq!(&set.tile_pixels(Tile::Water)[3..4], [WATER_MAX_ALPHA]);
    assert_eq!(
        &set.icon_pixels(Tile::GrassTop)[..3],
        [
            (200.0_f32 * 0x79 as f32 / 255.0).round() as u8,
            (200.0_f32 * 0xC0 as f32 / 255.0).round() as u8,
            (200.0_f32 * 0x5A as f32 / 255.0).round() as u8,
        ]
    );
}

#[test]
fn grass_side_gets_a_tinted_overlay() {
    let bytes = zip(&[
        (&block("grass_side.png"), png(16, 16, [100, 100, 100, 255])),
        (
            &block("grass_side_overlay.png"),
            png(16, 16, [255, 255, 255, 255]),
        ),
    ]);
    let set = load(&bytes).unwrap().textures;
    assert_eq!(set.tile(Tile::GrassSide).tint, WHITE);
    assert_eq!(&set.tile_pixels(Tile::GrassSide)[..3], [0x79, 0xC0, 0x5A]);
}

#[test]
fn animation_from_mcmeta() {
    let mut water = png(16, 16 * 4, [5, 5, 5, 255]);
    let meta =
        br#"{"animation":{"frametime":2,"interpolate":true,"frames":[0,{"index":2,"time":4},1]}}"#;
    let bytes = zip(&[
        (&block("water_still.png"), std::mem::take(&mut water)),
        (&block("water_still.png.mcmeta"), meta.to_vec()),
        (&block("lava_still.png"), png(16, 16 * 3, [9, 9, 9, 255])),
    ]);
    let set = load(&bytes).unwrap().textures;
    let water = set.tile(Tile::Water);
    assert_eq!(water.frames, 1 + 2 + 1);
    assert_eq!(water.fps, 10.0);
    assert!(water.interpolate);
    assert_eq!(water.scroll, 0.0);
    let lava = set.tile(Tile::Lava);
    assert_eq!(lava.frames, 3);
    assert_eq!(lava.fps, 20.0);
    assert!(!lava.interpolate);
}

#[test]
fn custom_frame_order_picks_the_right_images() {
    let mut image = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut image, 16, 32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        let data = [[1, 0, 0, 255].repeat(256), [2, 0, 0, 255].repeat(256)].concat();
        writer.write_image_data(&data).unwrap();
        writer.finish().unwrap();
    }
    let bytes = zip(&[
        (&block("lava_still.png"), image),
        (
            &block("lava_still.png.mcmeta"),
            br#"{"animation":{"frames":[1,0,1]}}"#.to_vec(),
        ),
    ]);
    let set = load(&bytes).unwrap().textures;
    let lava = set.tile(Tile::Lava);
    let reds: Vec<u8> = lava
        .layers
        .chunks(set.frame_bytes())
        .map(|f| f[0])
        .collect();
    assert_eq!(reds, [2, 1, 2]);
}

#[test]
fn bad_mcmeta_keeps_default_animation() {
    let bytes = zip(&[
        (&block("lava_still.png"), png(16, 16 * 2, [9, 9, 9, 255])),
        (&block("lava_still.png.mcmeta"), b"{oops".to_vec()),
    ]);
    let lava_frames = load(&bytes).unwrap().textures.tile(Tile::Lava).frames;
    assert_eq!(lava_frames, 2);
}

#[test]
fn reads_pack_meta_and_nested_root() {
    let bytes = zip(&[
        (
            "MyPack/pack.mcmeta",
            r#"{"pack":{"pack_format":3,"description":"§aFaith§rful"}}"#
                .as_bytes()
                .to_vec(),
        ),
        ("MyPack/pack.png", vec![1, 2, 3]),
        (
            "MyPack/assets/minecraft/textures/blocks/dirt.png",
            png(16, 16, [1, 2, 3, 255]),
        ),
    ]);
    let pack = load(&bytes).unwrap();
    assert_eq!(pack.description, "Faithful");
    assert_eq!(pack.icon, Some(vec![1, 2, 3]));
    assert_eq!(&pack.textures.tile_pixels(Tile::Dirt)[..3], [1, 2, 3]);
}

#[test]
fn strips_formatting_codes() {
    assert_eq!(strip_formatting("§1A§lB§r C"), "AB C");
    assert_eq!(strip_formatting("plain"), "plain");
}

#[test]
fn rejects_too_many_layers() {
    let bytes = zip(&[(&block("lava_still.png"), png(16, 16 * 300, [9, 9, 9, 255]))]);
    assert!(load(&bytes).is_err());
}

#[test]
fn non_power_of_two_size_rounds_up() {
    let bytes = zip(&[(&block("dirt.png"), png(24, 24, [1, 2, 3, 255]))]);
    assert_eq!(load(&bytes).unwrap().textures.size(), 32);
}
