use std::io::Write;

use super::*;
use crate::biome::Biome;

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
    assert_eq!(standard.size(), 32);
    assert_eq!(
        set.tile_pixels(Tile::Stone),
        standard.tile_pixels(Tile::Stone)
    );
    assert_eq!(set.tile(Tile::Stone).tint, WHITE);
    assert_eq!(set.tile(Tile::TallGrass).biome, BiomeTint::Grass);
}

#[test]
fn layer_count_matches_frames() {
    let bytes = zip(&[(&block("dirt.png"), png(16, 16, [1, 1, 1, 255]))]);
    let set = load(&bytes).unwrap().textures;
    let total: usize = Tile::ALL
        .iter()
        .map(|&t| set.tile(t).layer_span() as usize)
        .sum();
    assert_eq!(set.layer_count() as usize, total);
    for tile in Tile::ALL {
        let texture = set.tile(tile);
        assert_eq!(
            texture.layers.len(),
            texture.layer_span() as usize * set.frame_bytes()
        );
    }
}

#[test]
fn fixed_tints_and_biome_kinds() {
    let bytes = zip(&[
        (&block("grass_top.png"), png(16, 16, [200, 200, 200, 255])),
        (&block("leaves_oak.png"), png(16, 16, [200, 200, 200, 255])),
        (
            &block("leaves_birch.png"),
            png(16, 16, [200, 200, 200, 255]),
        ),
        (&block("water_still.png"), png(16, 16, [200, 200, 200, 255])),
    ]);
    let set = load(&bytes).unwrap().textures;
    let hex = |c: [f32; 3]| c.map(|v| (v * 255.0).round() as u8);
    assert_eq!(set.tile(Tile::GrassTop).biome, BiomeTint::Grass);
    assert_eq!(set.tile(Tile::GrassTop).tint, WHITE);
    assert_eq!(set.tile(Tile::Leaves).biome, BiomeTint::Foliage);
    assert_eq!(set.tile(Tile::TallGrass).biome, BiomeTint::Grass);
    assert_eq!(set.tile(Tile::BirchLeaves).biome, BiomeTint::None);
    assert_eq!(hex(set.tile(Tile::BirchLeaves).tint), [0x80, 0xA7, 0x55]);
    for tile in [
        Tile::JungleLeaves,
        Tile::AcaciaLeaves,
        Tile::DarkOakLeaves,
        Tile::Vine,
    ] {
        assert_eq!(set.tile(tile).biome, BiomeTint::Foliage, "{tile:?}");
    }
    assert_eq!(set.tile(Tile::MelonSide).biome, BiomeTint::None);
    assert_eq!(set.tile(Tile::Waterlily).biome, BiomeTint::None);
    assert_eq!(hex(set.tile(Tile::Waterlily).tint), [0x20, 0x80, 0x30]);
    assert_eq!(hex(set.tile(Tile::Water).tint), [0x3F, 0x76, 0xE4]);
    assert_eq!(&set.tile_pixels(Tile::Water)[3..4], [WATER_MAX_ALPHA]);
    let plains = BiomePalette::constants().plains(BiomeTint::Grass);
    let icon = set.icon_pixels(Tile::GrassTop);
    for c in 0..3 {
        assert_eq!(icon[c], (200.0_f32 * plains[c]).round() as u8);
    }
}

#[test]
fn grass_side_overlay_is_a_separate_layer() {
    let bytes = zip(&[
        (&block("grass_side.png"), png(16, 16, [100, 100, 100, 255])),
        (
            &block("grass_side_overlay.png"),
            png(16, 16, [255, 255, 255, 255]),
        ),
    ]);
    let set = load(&bytes).unwrap().textures;
    let side = set.tile(Tile::GrassSide);
    assert!(side.overlay);
    assert_eq!(side.frames, 1);
    assert_eq!(side.layers.len(), 2 * set.frame_bytes());
    assert_eq!(&set.tile_pixels(Tile::GrassSide)[..3], [100, 100, 100]);
    assert_eq!(side.biome, BiomeTint::None);
    let plains = BiomePalette::constants().plains(BiomeTint::Grass);
    let icon = set.icon_pixels(Tile::GrassSide);
    for c in 0..3 {
        assert_eq!(icon[c], (255.0_f32 * plains[c]).round() as u8);
    }
    let table = set.animation_table();
    let entry = table[Tile::GrassSide.layer() as usize];
    assert_eq!(entry[1][2], entry[1][0] + 1.0);
    assert_eq!(entry[1][3], 1.0);
    assert_eq!(set.first_layer(Tile::Dirt), entry[1][0] as u32 + 2);
}

#[test]
fn colormaps_override_constants_and_default_without_them() {
    let none = zip(&[(&block("dirt.png"), png(16, 16, [1, 2, 3, 255]))]);
    assert_eq!(
        *load(&none).unwrap().textures.palette(),
        BiomePalette::constants()
    );
    let map = |name: &str, rgba: [u8; 4]| {
        (
            format!("assets/minecraft/textures/colormap/{name}.png"),
            png(256, 256, rgba),
        )
    };
    let (grass_name, grass) = map("grass", [10, 20, 30, 255]);
    let with = zip(&[
        (&block("dirt.png"), png(16, 16, [1, 2, 3, 255])),
        (&grass_name, grass),
    ]);
    let palette = load(&with).unwrap().textures.palette().clone();
    let expected = [10.0 / 255.0, 20.0 / 255.0, 30.0 / 255.0];
    assert_eq!(palette.grass(Biome::Forest), expected);
    assert_eq!(palette.grass(Biome::Plains), expected);
    assert_eq!(
        palette.foliage(Biome::Forest),
        BiomePalette::constants().foliage(Biome::Forest)
    );
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

#[test]
fn smaller_packs_downscale_missing_builtin_tiles() {
    let bytes = zip(&[(&block("dirt.png"), png(16, 16, [1, 2, 3, 255]))]);
    let set = load(&bytes).unwrap().textures;
    assert_eq!(set.size(), 16);
    assert_eq!(set.tile_pixels(Tile::Stone).len(), 16 * 16 * 4);
}

#[test]
fn builtin_pack_finds_every_tile_without_fallback() {
    let pack = builtin();
    assert_eq!(pack.textures.size(), 32);
    for tile in Tile::ALL {
        let name = source(tile).0;
        assert!(
            builtin_file(&format!("{BLOCKS_DIR}{name}.png")).is_some(),
            "{tile:?}: {name}.png is not built in"
        );
    }
}

#[test]
fn builtin_files_are_exactly_the_mapped_ones() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/textures");
    let mut on_disk: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|name| !["LICENSE.txt", "gui", "entity", "items"].contains(&name.as_str()))
        .collect();
    on_disk.sort();
    let mut built_in: Vec<String> = BUILTIN.iter().map(|(name, _)| name.to_string()).collect();
    built_in.sort();
    assert_eq!(on_disk, built_in);
}

const GUI_PREFIX: &str = "assets/minecraft/textures/gui/";

fn gui_zip(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut all = vec![(block("dirt.png"), png(16, 16, [1, 2, 3, 255]))];
    all.extend(
        files
            .iter()
            .map(|(n, b)| (format!("{GUI_PREFIX}{n}"), b.clone())),
    );
    let refs: Vec<(&str, Vec<u8>)> = all.iter().map(|(n, b)| (n.as_str(), b.clone())).collect();
    zip(&refs)
}

#[test]
fn gui_sheets_come_from_the_pack() {
    let widgets = png(64, 64, [9, 8, 7, 255]);
    let bytes = gui_zip(&[("widgets.png", widgets.clone())]);
    let gui = load(&bytes).unwrap().gui;
    assert_eq!(gui.widgets, widgets);
}

#[test]
fn gui_falls_back_to_builtin_when_missing_or_broken() {
    let bytes = gui_zip(&[("icons.png", vec![1, 2, 3])]);
    let gui = load(&bytes).unwrap().gui;
    let builtin = builtin_gui();
    assert_eq!(gui.widgets, builtin.widgets);
    assert_eq!(gui.icons, builtin.icons);
    assert_eq!(gui.inventory, builtin.inventory);
}

#[test]
fn non_square_gui_sheet_is_ignored() {
    let bytes = gui_zip(&[("widgets.png", png(64, 32, [0, 0, 0, 255]))]);
    assert_eq!(load(&bytes).unwrap().gui.widgets, builtin_gui().widgets);
}

#[test]
fn panel_is_three_by_three_cells_sized_by_the_sheet() {
    let gui = builtin_gui();
    assert_eq!(gui.panel_slice, 8);
    let image = decode_png(&gui.panel).unwrap();
    assert_eq!((image.width, image.height), (24, 24));
    let small = gui_zip(&[("container/inventory.png", png(64, 64, [5, 6, 7, 255]))]);
    let gui = load(&small).unwrap().gui;
    assert_eq!(gui.panel_slice, 1);
    let image = decode_png(&gui.panel).unwrap();
    assert_eq!((image.width, image.height), (3, 3));
    assert_eq!(&image.rgba[..4], [5, 6, 7, 255]);
}

#[test]
fn builtin_gui_files_are_exactly_the_mapped_ones() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/textures/gui");
    let mut on_disk = Vec::new();
    let mut stack = vec![dir.clone()];
    while let Some(path) = stack.pop() {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                on_disk.push(
                    path.strip_prefix(&dir)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string(),
                );
            }
        }
    }
    on_disk.sort();
    let mut built_in: Vec<String> = BUILTIN_GUI
        .iter()
        .map(|(name, _)| name.strip_prefix("gui/").unwrap().to_string())
        .collect();
    built_in.sort();
    assert_eq!(on_disk, built_in);
}

#[test]
fn builtin_entity_sheets_are_two_to_one() {
    let entities = builtin_entities();
    for sheet in [
        &entities.pig,
        &entities.chicken,
        &entities.cow,
        &entities.sheep,
        &entities.sheep_fur,
    ] {
        assert!(sheet.width >= 64 && sheet.width == 2 * sheet.height);
    }
}
