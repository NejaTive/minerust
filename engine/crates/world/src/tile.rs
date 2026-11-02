pub const TILE_SIZE: u32 = 16;

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
    Gravel,
    Lava,
    Obsidian,
    Cobblestone,
    BirchLogSide,
    BirchLogTop,
    BirchLeaves,
    SpruceLogSide,
    SpruceLogTop,
    SpruceLeaves,
    Bedrock,
    SnowyGrassSide,
    Snow,
    Ice,
}

impl Tile {
    pub const ALL: [Tile; 28] = [
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
        Tile::Gravel,
        Tile::Lava,
        Tile::Obsidian,
        Tile::Cobblestone,
        Tile::BirchLogSide,
        Tile::BirchLogTop,
        Tile::BirchLeaves,
        Tile::SpruceLogSide,
        Tile::SpruceLogTop,
        Tile::SpruceLeaves,
        Tile::Bedrock,
        Tile::SnowyGrassSide,
        Tile::Snow,
        Tile::Ice,
    ];

    pub const fn frames(self) -> u32 {
        match self {
            Tile::Water | Tile::Lava => 16,
            _ => 1,
        }
    }

    pub fn fps(self) -> f32 {
        match self {
            Tile::Water => 10.0,
            Tile::Lava => 5.0,
            _ => 0.0,
        }
    }

    pub fn scroll(self) -> f32 {
        match self {
            Tile::Water => 1.0,
            Tile::Lava => 0.3,
            _ => 0.0,
        }
    }

    pub fn sway(self) -> f32 {
        match self {
            Tile::Leaves | Tile::BirchLeaves | Tile::SpruceLeaves => 1.0,
            Tile::TallGrass | Tile::Poppy | Tile::Dandelion => 2.0,
            _ => 0.0,
        }
    }

    pub fn emissive(self) -> bool {
        matches!(self, Tile::Glowstone | Tile::Lava | Tile::Torch)
    }

    pub fn layer(self) -> u32 {
        Tile::ALL
            .iter()
            .take_while(|&&t| t != self)
            .map(|t| t.frames())
            .sum()
    }

    pub fn is_cutout(self) -> bool {
        matches!(
            self,
            Tile::Leaves
                | Tile::BirchLeaves
                | Tile::SpruceLeaves
                | Tile::TallGrass
                | Tile::Poppy
                | Tile::Dandelion
                | Tile::Torch
        )
    }
}

/// Прибавляется к колыханию в `.w` записи таблицы анимации; читает `shader.wgsl`.
pub const EMISSIVE_FLAG: f32 = 4.0;

pub const LAYER_COUNT: u32 = {
    let mut sum = 0;
    let mut i = 0;
    while i < Tile::ALL.len() {
        sum += Tile::ALL[i].frames();
        i += 1;
    }
    sum
};

pub fn animation_table() -> Vec<[f32; 4]> {
    Tile::ALL
        .iter()
        .flat_map(|&t| {
            let flags = t.sway() + if t.emissive() { EMISSIVE_FLAG } else { 0.0 };
            let entry = [t.frames() as f32, t.fps(), t.scroll(), flags];
            (0..t.frames()).map(move |_| entry)
        })
        .collect()
}

pub const TORCH_PIXELS: (u32, u32) = (7, 9);
pub const TORCH_TOP_ROW: u32 = 6;

impl Tile {
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
            Torch => "torch", Gravel => "gravel", Lava => "lava", Obsidian => "obsidian",
            Cobblestone => "cobblestone", BirchLogSide => "birch_log_side",
            BirchLogTop => "birch_log_top", BirchLeaves => "birch_leaves",
            SpruceLogSide => "spruce_log_side", SpruceLogTop => "spruce_log_top",
            SpruceLeaves => "spruce_leaves", Bedrock => "bedrock",
            SnowyGrassSide => "snowy_grass_side", Snow => "snow", Ice => "ice",
        }
    }
}

pub fn tile_pixels(tile: Tile) -> Vec<u8> {
    let mut frames = tile_frames(tile);
    frames.truncate((TILE_SIZE * TILE_SIZE * 4) as usize);
    frames
}

pub fn tile_frames(tile: Tile) -> Vec<u8> {
    let name = tile.file_name();
    let mut decoder = png::Decoder::new(std::io::Cursor::new(tile.png()));
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
        info.width == TILE_SIZE && info.height == TILE_SIZE * tile.frames(),
        "{name}.png: expected {TILE_SIZE}x{}, got {}x{}",
        TILE_SIZE * tile.frames(),
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

#[cfg(test)]
mod tests;
