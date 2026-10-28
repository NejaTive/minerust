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

impl Tile {
    /// Имя файла в `engine/crates/world/assets/textures/`.
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

    /// PNG тайла, вшитый в бинарник. Файлы лежат в `engine/crates/world/assets/textures/<имя_в_snake_case>.png`;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Альфа во всех пикселях `water.png`: дно на глубине пары блоков ещё видно.
    const WATER_ALPHA: u8 = 180;

    #[test]
    fn layer_count_for_current_tiles() {
        assert_eq!(Tile::ALL.len(), 14);
        assert_eq!(LAYER_COUNT, 14);
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
}
