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
}

impl Tile {
    pub const ALL: [Tile; 18] = [
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
    ];

    pub fn layer(self) -> u32 {
        self as u32
    }

    pub fn is_cutout(self) -> bool {
        matches!(
            self,
            Tile::Leaves | Tile::TallGrass | Tile::Poppy | Tile::Dandelion | Tile::Torch
        )
    }
}

pub const LAYER_COUNT: u32 = Tile::ALL.len() as u32;

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
            Cobblestone => "cobblestone",
        }
    }
}

pub fn tile_pixels(tile: Tile) -> Vec<u8> {
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

#[cfg(test)]
mod tests;
