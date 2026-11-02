use crate::tile::{DEFAULT_TILE_SIZE, EMISSIVE_FLAG, Tile};

pub const MAX_LAYERS: usize = 256;
pub const WHITE: [f32; 3] = [1.0; 3];

pub struct TileTexture {
    pub layers: Vec<u8>,
    pub frames: u32,
    pub fps: f32,
    pub scroll: f32,
    pub tint: [f32; 3],
    pub interpolate: bool,
}

pub struct TextureSet {
    size: u32,
    tiles: Vec<TileTexture>,
}

pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

const MAX_IMAGE_BYTES: usize = 64 << 20;

pub fn decode_png(bytes: &[u8]) -> Result<DecodedImage, String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let size = reader
        .output_buffer_size()
        .filter(|&size| size <= MAX_IMAGE_BYTES)
        .ok_or("image is too large")?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    if info.bit_depth != png::BitDepth::Eight {
        return Err(format!("unsupported bit depth {:?}", info.bit_depth));
    }
    let data = &buf[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => data
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => data.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err("unexpanded palette".into()),
    };
    Ok(DecodedImage {
        width: info.width,
        height: info.height,
        rgba,
    })
}

pub fn scale_nearest(src: &[u8], src_size: u32, size: u32) -> Vec<u8> {
    if src_size == size {
        return src.to_vec();
    }
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        let sy = (u64::from(y) * u64::from(src_size) / u64::from(size)) as usize;
        for x in 0..size {
            let sx = (u64::from(x) * u64::from(src_size) / u64::from(size)) as usize;
            let i = (sy * src_size as usize + sx) * 4;
            out.extend_from_slice(&src[i..i + 4]);
        }
    }
    out
}

impl TileTexture {
    pub fn still(pixels: Vec<u8>) -> TileTexture {
        TileTexture {
            layers: pixels,
            frames: 1,
            fps: 0.0,
            scroll: 0.0,
            tint: WHITE,
            interpolate: false,
        }
    }
}

impl TextureSet {
    pub fn standard() -> TextureSet {
        let tiles = Tile::ALL.iter().map(|&tile| standard_tile(tile)).collect();
        TextureSet {
            size: DEFAULT_TILE_SIZE,
            tiles,
        }
    }

    pub fn from_tiles(size: u32, tiles: Vec<TileTexture>) -> TextureSet {
        assert_eq!(tiles.len(), Tile::ALL.len());
        TextureSet { size, tiles }
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn tile(&self, tile: Tile) -> &TileTexture {
        &self.tiles[tile.layer() as usize]
    }

    pub fn layer_count(&self) -> u32 {
        self.tiles.iter().map(|t| t.frames).sum()
    }

    pub fn first_layer(&self, tile: Tile) -> u32 {
        self.tiles[..tile.layer() as usize]
            .iter()
            .map(|t| t.frames)
            .sum()
    }

    pub fn frame_bytes(&self) -> usize {
        (self.size * self.size * 4) as usize
    }

    /// Запись на тайл: `[кадры, fps, прокрутка, флаги]`, `[первый слой, интерполяция, 0, 0]`,
    /// `[оттенок rgb, 0]`; раскладку читает `shader.wgsl` (`TileAnim`).
    pub fn animation_table(&self) -> Vec<[[f32; 4]; 3]> {
        let mut first = 0;
        Tile::ALL
            .iter()
            .map(|&tile| {
                let t = self.tile(tile);
                let flags = tile.sway() + if tile.emissive() { EMISSIVE_FLAG } else { 0.0 };
                let entry = [
                    [t.frames as f32, t.fps, t.scroll, flags],
                    [first as f32, f32::from(u8::from(t.interpolate)), 0.0, 0.0],
                    [t.tint[0], t.tint[1], t.tint[2], 0.0],
                ];
                first += t.frames;
                entry
            })
            .collect()
    }

    pub fn tile_pixels(&self, tile: Tile) -> Vec<u8> {
        self.tile(tile).layers[..self.frame_bytes()].to_vec()
    }

    pub fn icon_pixels(&self, tile: Tile) -> Vec<u8> {
        let tint = self.tile(tile).tint;
        let mut pixels = self.tile_pixels(tile);
        for p in pixels.as_chunks_mut::<4>().0 {
            for c in 0..3 {
                p[c] = (f32::from(p[c]) * tint[c]).round().min(255.0) as u8;
            }
        }
        pixels
    }
}

fn standard_tile(tile: Tile) -> TileTexture {
    let name = tile.file_name();
    let image = decode_png(tile.png()).unwrap_or_else(|e| panic!("{name}.png: {e}"));
    assert!(
        image.width == DEFAULT_TILE_SIZE && image.height == DEFAULT_TILE_SIZE * tile.frames(),
        "{name}.png: expected {DEFAULT_TILE_SIZE}x{}, got {}x{}",
        DEFAULT_TILE_SIZE * tile.frames(),
        image.width,
        image.height
    );
    TileTexture {
        layers: image.rgba,
        frames: tile.frames(),
        fps: tile.fps(),
        scroll: tile.scroll(),
        tint: WHITE,
        interpolate: false,
    }
}

#[cfg(test)]
mod tests;
