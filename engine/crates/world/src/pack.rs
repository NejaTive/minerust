use std::io::Read;

use serde_json::Value;

use crate::textures::{
    DecodedImage, MAX_LAYERS, TextureSet, TileTexture, WHITE, decode_png, scale_nearest,
};
use crate::tile::{DEFAULT_TILE_SIZE, Tile};

const BLOCKS_DIR: &str = "assets/minecraft/textures/blocks/";
const MAX_TILE_SIZE: u32 = 512;
const MAX_ENTRY_BYTES: u64 = 64 << 20;
const WATER_MAX_ALPHA: u8 = 180;

const GRASS: [f32; 3] = rgb(0x79C05A);
const OAK_LEAVES: [f32; 3] = rgb(0x59AE30);
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
}

fn source(tile: Tile) -> (&'static str, [f32; 3]) {
    match tile {
        Tile::GrassTop => ("grass_top", GRASS),
        Tile::GrassSide => ("grass_side", WHITE),
        Tile::Dirt => ("dirt", WHITE),
        Tile::Stone => ("stone", WHITE),
        Tile::Sand => ("sand", WHITE),
        Tile::LogSide => ("log_oak", WHITE),
        Tile::LogTop => ("log_oak_top", WHITE),
        Tile::Leaves => ("leaves_oak", OAK_LEAVES),
        Tile::Water => ("water_still", WATER),
        Tile::Glowstone => ("glowstone", WHITE),
        Tile::TallGrass => ("tallgrass", GRASS),
        Tile::Poppy => ("flower_rose", WHITE),
        Tile::Dandelion => ("flower_dandelion", WHITE),
        Tile::Torch => ("torch_on", WHITE),
        Tile::Gravel => ("gravel", WHITE),
        Tile::Lava => ("lava_still", WHITE),
        Tile::Obsidian => ("obsidian", WHITE),
        Tile::Cobblestone => ("cobblestone", WHITE),
        Tile::BirchLogSide => ("log_birch", WHITE),
        Tile::BirchLogTop => ("log_birch_top", WHITE),
        Tile::BirchLeaves => ("leaves_birch", BIRCH_LEAVES),
        Tile::SpruceLogSide => ("log_spruce", WHITE),
        Tile::SpruceLogTop => ("log_spruce_top", WHITE),
        Tile::SpruceLeaves => ("leaves_spruce", SPRUCE_LEAVES),
        Tile::Bedrock => ("bedrock", WHITE),
        Tile::SnowyGrassSide => ("grass_side_snowed", WHITE),
        Tile::Snow => ("snow", WHITE),
        Tile::Ice => ("ice", WHITE),
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

    let mut found: Vec<Option<Found>> = Vec::new();
    for &tile in &Tile::ALL {
        let stem = source(tile).0;
        let image = read(&format!("{BLOCKS_DIR}{stem}.png")).and_then(|b| decode_png(&b).ok());
        let meta = read(&format!("{BLOCKS_DIR}{stem}.png.mcmeta"));
        found.push(image.and_then(|image| {
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
        }));
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
    let size = widest.next_power_of_two().max(DEFAULT_TILE_SIZE);
    if size > MAX_TILE_SIZE {
        return Err(format!("textures are too large: {widest}px"));
    }

    let overlay = found[Tile::GrassSide.layer() as usize]
        .as_ref()
        .and_then(|_| read(&format!("{BLOCKS_DIR}grass_side_overlay.png")))
        .and_then(|b| decode_png(&b).ok())
        .filter(|image| image.width > 0 && image.height >= image.width);

    let standard = TextureSet::standard();
    let mut tiles = Vec::new();
    for (&tile, found) in Tile::ALL.iter().zip(&found) {
        let Some(found) = found else {
            tiles.push(scaled_standard(&standard, tile, size));
            continue;
        };
        let mut texture = build_tile(found, size);
        texture.tint = source(tile).1;
        if tile == Tile::GrassSide
            && let Some(overlay) = &overlay
        {
            apply_overlay(&mut texture, overlay, size);
        }
        if tile == Tile::Water {
            for p in texture.layers.as_chunks_mut::<4>().0 {
                p[3] = p[3].min(WATER_MAX_ALPHA);
            }
        }
        tiles.push(texture);
    }

    let layers: u32 = tiles.iter().map(|t| t.frames).sum();
    if layers as usize > MAX_LAYERS {
        return Err(format!(
            "too many animation frames: {layers} (limit {MAX_LAYERS})"
        ));
    }

    let (description, icon) = read_meta(&mut read);
    Ok(TexturePack {
        textures: TextureSet::from_tiles(size, tiles),
        description,
        icon,
    })
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
        scroll: 0.0,
        tint: WHITE,
        interpolate,
    }
}

fn scaled_standard(standard: &TextureSet, tile: Tile, size: u32) -> TileTexture {
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
        scroll: src.scroll,
        tint: WHITE,
        interpolate: false,
    }
}

fn apply_overlay(texture: &mut TileTexture, overlay: &DecodedImage, size: u32) {
    let width = overlay.width;
    let top = scale_nearest(&overlay.rgba[..(width * width * 4) as usize], width, size);
    let tint = GRASS;
    for layer in texture.layers.chunks_exact_mut((size * size * 4) as usize) {
        for (dst, src) in layer
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(top.as_chunks::<4>().0)
        {
            let alpha = f32::from(src[3]) / 255.0;
            for c in 0..3 {
                let tinted = f32::from(src[c]) * tint[c];
                dst[c] = (f32::from(dst[c]) * (1.0 - alpha) + tinted * alpha).round() as u8;
            }
            dst[3] = dst[3].max(src[3]);
        }
    }
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
