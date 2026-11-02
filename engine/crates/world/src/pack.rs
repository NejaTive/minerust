use std::io::Read;

use serde_json::Value;

use crate::biome_color::{BiomePalette, BiomeTint, Colormap};
use crate::textures::{
    DecodedImage, MAX_LAYERS, TextureSet, TileTexture, WHITE, decode_png, scale_nearest,
};
use crate::tile::Tile;

const BLOCKS_DIR: &str = "assets/minecraft/textures/blocks/";
const GUI_DIR: &str = "assets/minecraft/textures/gui/";
const GUI_WIDGETS: &str = "widgets.png";
const GUI_ICONS: &str = "icons.png";
const GUI_INVENTORY: &str = "container/inventory.png";
const GUI_SIZE_UNITS: u32 = 256;
const PANEL_CORNER_UNITS: u32 = 4;
const PANEL_WIDTH_UNITS: u32 = 176;
const PANEL_HEIGHT_UNITS: u32 = 166;
const PANEL_FLAT_X_UNITS: u32 = 80;
const PANEL_FLAT_Y_UNITS: u32 = 30;
const PANEL_EDGE_UNITS: u32 = 80;
const COLORMAP_DIR: &str = "assets/minecraft/textures/colormap/";
const MIN_TILE_SIZE: u32 = 16;
const MAX_TILE_SIZE: u32 = 512;
const MAX_ENTRY_BYTES: u64 = 64 << 20;
const WATER_MAX_ALPHA: u8 = 180;

const BIRCH_LEAVES: [f32; 3] = rgb(0x80A755);
const SPRUCE_LEAVES: [f32; 3] = rgb(0x619961);
const WATER: [f32; 3] = rgb(0x3F76E4);

const fn rgb(hex: u32) -> [f32; 3] {
    [
        ((hex >> 16) & 0xFF) as f32 / 255.0,
        ((hex >> 8) & 0xFF) as f32 / 255.0,
        (hex & 0xFF) as f32 / 255.0,
    ]
}

pub struct TexturePack {
    pub textures: TextureSet,
    pub description: String,
    pub icon: Option<Vec<u8>>,
    pub gui: GuiImages,
}

/// PNG-листы интерфейса; координаты в них считаются в единицах 256×256 (`width / 256` пикселей на единицу).
#[derive(Clone)]
pub struct GuiImages {
    pub widgets: Vec<u8>,
    pub icons: Vec<u8>,
    pub inventory: Vec<u8>,
    /// Девять квадратов 3×3 из углов, краёв и фона панели инвентаря, для `border-image`.
    pub panel: Vec<u8>,
    pub panel_slice: u32,
}

fn source(tile: Tile) -> (&'static str, [f32; 3], BiomeTint) {
    match tile {
        Tile::GrassTop => ("grass_top", WHITE, BiomeTint::Grass),
        Tile::GrassSide => ("grass_side", WHITE, BiomeTint::None),
        Tile::Dirt => ("dirt", WHITE, BiomeTint::None),
        Tile::Stone => ("stone", WHITE, BiomeTint::None),
        Tile::Sand => ("sand", WHITE, BiomeTint::None),
        Tile::LogSide => ("log_oak", WHITE, BiomeTint::None),
        Tile::LogTop => ("log_oak_top", WHITE, BiomeTint::None),
        Tile::Leaves => ("leaves_oak", WHITE, BiomeTint::Foliage),
        Tile::Water => ("water_still", WATER, BiomeTint::None),
        Tile::Glowstone => ("glowstone", WHITE, BiomeTint::None),
        Tile::TallGrass => ("tallgrass", WHITE, BiomeTint::Grass),
        Tile::Poppy => ("flower_rose", WHITE, BiomeTint::None),
        Tile::Dandelion => ("flower_dandelion", WHITE, BiomeTint::None),
        Tile::Torch => ("torch_on", WHITE, BiomeTint::None),
        Tile::Gravel => ("gravel", WHITE, BiomeTint::None),
        Tile::Lava => ("lava_still", WHITE, BiomeTint::None),
        Tile::Obsidian => ("obsidian", WHITE, BiomeTint::None),
        Tile::Cobblestone => ("cobblestone", WHITE, BiomeTint::None),
        Tile::BirchLogSide => ("log_birch", WHITE, BiomeTint::None),
        Tile::BirchLogTop => ("log_birch_top", WHITE, BiomeTint::None),
        Tile::BirchLeaves => ("leaves_birch", BIRCH_LEAVES, BiomeTint::None),
        Tile::SpruceLogSide => ("log_spruce", WHITE, BiomeTint::None),
        Tile::SpruceLogTop => ("log_spruce_top", WHITE, BiomeTint::None),
        Tile::SpruceLeaves => ("leaves_spruce", SPRUCE_LEAVES, BiomeTint::None),
        Tile::Bedrock => ("bedrock", WHITE, BiomeTint::None),
        Tile::SnowyGrassSide => ("grass_side_snowed", WHITE, BiomeTint::None),
        Tile::Snow => ("snow", WHITE, BiomeTint::None),
        Tile::Ice => ("ice", WHITE, BiomeTint::None),
    }
}

struct Animation {
    sequence: Vec<usize>,
    ticks: u32,
    interpolate: bool,
}

struct Found {
    image: DecodedImage,
    frames: usize,
    animation: Option<Animation>,
}

pub fn load(zip_bytes: &[u8]) -> Result<TexturePack, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes))
        .map_err(|e| format!("not a zip archive: {e}"))?;
    let root = archive
        .file_names()
        .filter_map(Result::ok)
        .find_map(|name| name.find(BLOCKS_DIR).map(|at| name[..at].to_string()))
        .ok_or("the archive has no assets/minecraft/textures/blocks")?;
    let mut read = |path: &str| -> Option<Vec<u8>> {
        let file = archive.by_name(&format!("{root}{path}")).ok()?;
        let mut out = Vec::new();
        file.take(MAX_ENTRY_BYTES).read_to_end(&mut out).ok()?;
        Some(out)
    };
    let textures = build(&mut read, Some(&builtin().textures))?;
    let (description, icon) = read_meta(&mut read);
    let gui = gui_images(&mut read);
    Ok(TexturePack {
        textures,
        description,
        icon,
        gui,
    })
}

pub fn builtin() -> TexturePack {
    let textures = build(&mut |path| builtin_file(path).map(<[u8]>::to_vec), None)
        .expect("built-in textures are complete");
    TexturePack {
        textures,
        description: "Faithful 32x".into(),
        icon: None,
        gui: builtin_gui(),
    }
}

pub fn builtin_gui() -> GuiImages {
    gui_images(&mut |_| None)
}

fn gui_images(read: &mut dyn FnMut(&str) -> Option<Vec<u8>>) -> GuiImages {
    let mut sheet = |name: &str| {
        read(&format!("{GUI_DIR}{name}"))
            .and_then(|bytes| {
                let image = decode_png(&bytes).ok()?;
                (image.width >= MIN_TILE_SIZE && image.width == image.height)
                    .then_some((bytes, image))
            })
            .unwrap_or_else(|| {
                let bytes = builtin_gui_file(name).expect("built-in gui sheets are complete");
                let image = decode_png(bytes).expect("built-in gui sheets are valid");
                (bytes.to_vec(), image)
            })
    };
    let (widgets, _) = sheet(GUI_WIDGETS);
    let (icons, _) = sheet(GUI_ICONS);
    let (inventory, image) = sheet(GUI_INVENTORY);
    let (panel, panel_slice) = panel_image(&image);
    GuiImages {
        widgets,
        icons,
        inventory,
        panel,
        panel_slice,
    }
}

fn panel_image(sheet: &DecodedImage) -> (Vec<u8>, u32) {
    let width = sheet.width;
    let unit = |v: u32| v * width / GUI_SIZE_UNITS;
    let corner = unit(PANEL_CORNER_UNITS).max(1);
    let right = unit(PANEL_WIDTH_UNITS) - corner;
    let bottom = unit(PANEL_HEIGHT_UNITS) - corner;
    let edge = unit(PANEL_EDGE_UNITS);
    let out = corner * 3;
    let mut rgba = vec![0u8; (out * out * 4) as usize];
    let cells = [
        ((0, 0), (0, 0)),
        ((1, 0), (edge, 0)),
        ((2, 0), (right, 0)),
        ((0, 1), (0, edge)),
        ((1, 1), (unit(PANEL_FLAT_X_UNITS), unit(PANEL_FLAT_Y_UNITS))),
        ((2, 1), (right, edge)),
        ((0, 2), (0, bottom)),
        ((1, 2), (edge, bottom)),
        ((2, 2), (right, bottom)),
    ];
    for ((cx, cy), (sx, sy)) in cells {
        for y in 0..corner {
            for x in 0..corner {
                let from = (((sy + y) * width + sx + x) * 4) as usize;
                let to = (((cy * corner + y) * out + cx * corner + x) * 4) as usize;
                rgba[to..to + 4].copy_from_slice(&sheet.rgba[from..from + 4]);
            }
        }
    }
    (encode_png(out, &rgba), corner)
}

fn encode_png(size: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, size, size);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("png header");
    writer.write_image_data(rgba).expect("png data");
    writer.finish().expect("png end");
    out
}

macro_rules! builtin_files {
    ($($file:literal),* $(,)?) => {
        &[$(($file, include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"), "/assets/textures/", $file
        )))),*]
    };
}

const BUILTIN: &[(&str, &[u8])] = builtin_files![
    "grass_top.png",
    "grass_side.png",
    "grass_side_overlay.png",
    "dirt.png",
    "stone.png",
    "sand.png",
    "log_oak.png",
    "log_oak_top.png",
    "leaves_oak.png",
    "water_still.png",
    "water_still.png.mcmeta",
    "glowstone.png",
    "tallgrass.png",
    "flower_rose.png",
    "flower_dandelion.png",
    "torch_on.png",
    "gravel.png",
    "lava_still.png",
    "lava_still.png.mcmeta",
    "obsidian.png",
    "cobblestone.png",
    "log_birch.png",
    "log_birch_top.png",
    "leaves_birch.png",
    "log_spruce.png",
    "log_spruce_top.png",
    "leaves_spruce.png",
    "bedrock.png",
    "grass_side_snowed.png",
    "snow.png",
    "ice.png",
];

fn builtin_file(path: &str) -> Option<&'static [u8]> {
    let name = path.strip_prefix(BLOCKS_DIR)?;
    BUILTIN
        .iter()
        .find(|(file, _)| *file == name)
        .map(|(_, bytes)| *bytes)
}

const BUILTIN_GUI: &[(&str, &[u8])] = builtin_files![
    "gui/widgets.png",
    "gui/icons.png",
    "gui/container/inventory.png",
];

fn builtin_gui_file(name: &str) -> Option<&'static [u8]> {
    BUILTIN_GUI
        .iter()
        .find(|(file, _)| file.strip_prefix("gui/") == Some(name))
        .map(|(_, bytes)| *bytes)
}

fn build(
    read: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
    fallback: Option<&TextureSet>,
) -> Result<TextureSet, String> {
    let mut found: Vec<Option<Found>> = Vec::new();
    for &tile in &Tile::ALL {
        let stem = source(tile).0;
        let image = read(&format!("{BLOCKS_DIR}{stem}.png")).and_then(|b| decode_png(&b).ok());
        let meta = read(&format!("{BLOCKS_DIR}{stem}.png.mcmeta"));
        let tile_found = image.and_then(|image| {
            if image.width == 0 || image.height < image.width {
                return None;
            }
            let frames = (image.height / image.width) as usize;
            let animation = meta.and_then(|m| parse_animation(&m, frames));
            Some(Found {
                image,
                frames,
                animation,
            })
        });
        if tile_found.is_none() && fallback.is_none() {
            return Err(format!("missing texture {stem}.png"));
        }
        found.push(tile_found);
    }
    if found.iter().all(Option::is_none) {
        return Err("the archive has no usable block textures".into());
    }

    let widest = found
        .iter()
        .flatten()
        .map(|f| f.image.width)
        .max()
        .unwrap_or(0);
    let size = widest.next_power_of_two().max(MIN_TILE_SIZE);
    if size > MAX_TILE_SIZE {
        return Err(format!("textures are too large: {widest}px"));
    }

    let overlay = found[Tile::GrassSide.layer() as usize]
        .as_ref()
        .and_then(|_| read(&format!("{BLOCKS_DIR}grass_side_overlay.png")))
        .and_then(|b| decode_png(&b).ok())
        .filter(|image| image.width > 0 && image.height >= image.width);

    let mut tiles = Vec::new();
    for (&tile, found) in Tile::ALL.iter().zip(&found) {
        let Some(found) = found else {
            let fallback = fallback.expect("checked while reading");
            tiles.push(scaled(fallback, tile, size));
            continue;
        };
        let mut texture = build_tile(found, size);
        let (_, tint, biome) = source(tile);
        texture.tint = tint;
        texture.biome = biome;
        if tile == Tile::GrassSide
            && let Some(overlay) = &overlay
        {
            add_overlay(&mut texture, overlay, size);
        }
        if tile == Tile::Water {
            for p in texture.layers.as_chunks_mut::<4>().0 {
                p[3] = p[3].min(WATER_MAX_ALPHA);
            }
        }
        tiles.push(texture);
    }

    let layers: u32 = tiles.iter().map(TileTexture::layer_span).sum();
    if layers as usize > MAX_LAYERS {
        return Err(format!(
            "too many animation frames: {layers} (limit {MAX_LAYERS})"
        ));
    }
    let mut colormap = |name: &str| {
        read(&format!("{COLORMAP_DIR}{name}.png"))
            .and_then(|b| decode_png(&b).ok())
            .and_then(|image| Colormap::from_image(&image))
    };
    let palette =
        BiomePalette::from_colormaps(colormap("grass").as_ref(), colormap("foliage").as_ref());
    Ok(TextureSet::from_tiles(size, tiles, palette))
}

fn build_tile(found: &Found, size: u32) -> TileTexture {
    let width = found.image.width;
    let frame_bytes = (width * width * 4) as usize;
    let frame = |index: usize| {
        scale_nearest(
            &found.image.rgba[index * frame_bytes..(index + 1) * frame_bytes],
            width,
            size,
        )
    };
    let (sequence, ticks, interpolate) = match &found.animation {
        Some(a) => (a.sequence.clone(), a.ticks, a.interpolate),
        None => ((0..found.frames).collect(), 1, false),
    };
    if sequence.len() == 1 {
        return TileTexture::still(frame(sequence[0]));
    }
    animated(&frame, &sequence, ticks, interpolate)
}

fn animated(
    frame: &impl Fn(usize) -> Vec<u8>,
    sequence: &[usize],
    ticks: u32,
    interpolate: bool,
) -> TileTexture {
    TileTexture {
        layers: sequence.iter().flat_map(|&i| frame(i)).collect(),
        frames: sequence.len() as u32,
        fps: 20.0 / ticks as f32,
        tint: WHITE,
        interpolate,
        biome: BiomeTint::None,
        overlay: false,
    }
}

fn scaled(standard: &TextureSet, tile: Tile, size: u32) -> TileTexture {
    let src = standard.tile(tile);
    let from = standard.size();
    let bytes = standard.frame_bytes();
    TileTexture {
        layers: src
            .layers
            .chunks_exact(bytes)
            .flat_map(|f| scale_nearest(f, from, size))
            .collect(),
        frames: src.frames,
        fps: src.fps,
        tint: src.tint,
        interpolate: src.interpolate,
        biome: src.biome,
        overlay: src.overlay,
    }
}

fn add_overlay(texture: &mut TileTexture, overlay: &DecodedImage, size: u32) {
    let width = overlay.width;
    let top = scale_nearest(&overlay.rgba[..(width * width * 4) as usize], width, size);
    texture.layers.extend_from_slice(&top);
    texture.overlay = true;
}

fn parse_animation(mcmeta: &[u8], frames: usize) -> Option<Animation> {
    let json: Value = serde_json::from_slice(mcmeta).ok()?;
    let anim = json.get("animation")?;
    let default_time = anim
        .get("frametime")
        .and_then(Value::as_u64)
        .map_or(1, |t| t.clamp(1, 1 << 16) as u32);
    let interpolate = anim
        .get("interpolate")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let entries: Vec<(usize, u32)> = match anim.get("frames").and_then(Value::as_array) {
        Some(list) => list
            .iter()
            .filter_map(|item| match item {
                Value::Number(n) => Some((n.as_u64()? as usize, default_time)),
                Value::Object(o) => {
                    let time = o
                        .get("time")
                        .and_then(Value::as_u64)
                        .map_or(default_time, |t| t.clamp(1, 1 << 16) as u32);
                    Some((o.get("index")?.as_u64()? as usize, time))
                }
                _ => None,
            })
            .filter(|&(index, _)| index < frames)
            .collect(),
        None => (0..frames).map(|i| (i, default_time)).collect(),
    };
    if entries.is_empty() {
        return None;
    }
    let ticks = entries.iter().fold(0, |acc, &(_, t)| gcd(acc, t));
    let sequence = entries
        .iter()
        .flat_map(|&(index, time)| std::iter::repeat_n(index, (time / ticks) as usize))
        .collect();
    Some(Animation {
        sequence,
        ticks,
        interpolate,
    })
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn read_meta(read: &mut impl FnMut(&str) -> Option<Vec<u8>>) -> (String, Option<Vec<u8>>) {
    let description = read("pack.mcmeta")
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|json| match json.get("pack")?.get("description")? {
            Value::String(s) => Some(strip_formatting(s)),
            Value::Object(o) => Some(strip_formatting(o.get("text")?.as_str()?)),
            _ => None,
        })
        .unwrap_or_default();
    (description, read("pack.png"))
}

pub fn strip_formatting(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '§' {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests;
