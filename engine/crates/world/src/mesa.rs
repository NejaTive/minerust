use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};

use crate::block::Block;
use crate::stream::Stream;
use crate::terrain::{Biome, WATER_LEVEL};
use crate::trees::hash;

pub const LAYER_SPAN: usize = 80;

const BAND_COUNT: usize = 64;
const BAND_OFFSET_SCALE: f32 = 512.0;
const DEPTH_FREQUENCY: f32 = 0.05;
const DEPTH_SCALE: f32 = 8.0;
const PILLAR_FREQUENCY: f32 = 0.25;
const PILLAR_ROOF_FREQUENCY: f32 = 0.001953125;
const PILLAR_BASE: f32 = 64.0;
const FOREST_Y: i32 = 86;
const SALT_JITTER: u64 = 40;

pub struct Mesa {
    seed: i32,
    bands: [Block; BAND_COUNT],
    band_offset: FastNoiseLite,
    depth: FastNoiseLite,
    pillar: FastNoiseLite,
    roof: FastNoiseLite,
}

fn noise(seed: i32, kind: NoiseType, octaves: i32, frequency: f32) -> FastNoiseLite {
    let mut noise = FastNoiseLite::with_seed(seed);
    noise.set_noise_type(Some(kind));
    noise.set_fractal_type(Some(FractalType::FBm));
    noise.set_fractal_octaves(Some(octaves));
    noise.set_frequency(Some(frequency));
    noise
}

fn bands(seed: i32) -> [Block; BAND_COUNT] {
    let mut bands = [Block::Terracotta; BAND_COUNT];
    let mut stream = Stream::new(u64::from(seed as u32) ^ 0x4D65_7361);
    let mut i = 0;
    while i < BAND_COUNT {
        i += stream.below(5) as usize + 1;
        if i < BAND_COUNT {
            bands[i] = Block::OrangeTerracotta;
        }
        i += 1;
    }
    for (color, thickness_floor, thickness_span) in [
        (Block::YellowTerracotta, 1, 3),
        (Block::BrownTerracotta, 2, 3),
        (Block::RedTerracotta, 1, 3),
    ] {
        for _ in 0..stream.below(4) + 2 {
            let length = (stream.below(thickness_span) + thickness_floor) as usize;
            let start = stream.below(BAND_COUNT as u32) as usize;
            for band in bands.iter_mut().skip(start).take(length) {
                *band = color;
            }
        }
    }
    let mut at = 0;
    for _ in 0..stream.below(3) + 3 {
        at += stream.below(16) as usize + 4;
        if at >= BAND_COUNT {
            continue;
        }
        bands[at] = Block::WhiteTerracotta;
        if at > 1 && stream.flip() {
            bands[at - 1] = Block::SilverTerracotta;
        }
        if at < BAND_COUNT - 1 && stream.flip() {
            bands[at + 1] = Block::SilverTerracotta;
        }
    }
    bands
}

impl Mesa {
    pub fn new(seed: i32) -> Self {
        Self {
            seed,
            bands: bands(seed),
            band_offset: noise(seed ^ 0x51, NoiseType::Perlin, 1, 1.0),
            depth: noise(seed ^ 0x52, NoiseType::OpenSimplex2, 4, DEPTH_FREQUENCY),
            pillar: noise(seed ^ 0x53, NoiseType::Perlin, 4, PILLAR_FREQUENCY),
            roof: noise(seed ^ 0x54, NoiseType::Perlin, 1, PILLAR_ROOF_FREQUENCY),
        }
    }

    pub fn depth_noise(&self, x: i32, z: i32) -> f32 {
        self.depth.get_noise_2d(x as f32, z as f32) * DEPTH_SCALE
    }

    pub fn band(&self, x: i32, y: i32) -> Block {
        let at = x as f32 / BAND_OFFSET_SCALE;
        let offset = (self.band_offset.get_noise_2d(at, at) * 2.0).round() as i32;
        self.bands[(y + offset + BAND_COUNT as i32).rem_euclid(BAND_COUNT as i32) as usize]
    }

    /// Верхний блок бриче-пика в колонке (0, если пика нет), см. `BiomeMesa.genTerrainBlocks`.
    pub fn pillar_top(&self, x: i32, z: i32) -> i32 {
        let pillar = self.pillar.get_noise_2d(x as f32, z as f32) * DEPTH_SCALE;
        let reach = self.depth_noise(x, z).abs().min(pillar);
        if reach <= 0.0 {
            return 0;
        }
        let roof = self.roof.get_noise_2d(x as f32, z as f32).abs();
        let ceiling = (roof * 50.0).ceil() + 14.0;
        let height = (reach * reach * 2.5).min(ceiling) + PILLAR_BASE;
        height as i32 - 1
    }

    /// Блоки колонки сверху вниз от `surface`; глубже [`LAYER_SPAN`] — камень.
    pub fn layers(
        &self,
        biome: Biome,
        surface: i32,
        x: i32,
        z: i32,
        out: &mut [Block; LAYER_SPAN],
    ) {
        let noise = self.depth_noise(x, z);
        let jitter = (hash(self.seed, x, z, SALT_JITTER) >> 8) as f32 / (1 << 24) as f32;
        let depth = (noise / 3.0 + 3.0 + jitter * 0.25) as i32;
        let striped = (noise / 3.0 * std::f32::consts::PI).cos() > 0.0;
        let forest = matches!(biome, Biome::MesaPlateauF | Biome::MesaPlateauFM);
        let mut left = -1;
        let mut sand_cap = false;
        for (d, slot) in out.iter_mut().enumerate() {
            let y = surface - d as i32;
            *slot = Block::Stone;
            if left == -1 {
                sand_cap = false;
                left = depth + (y - WATER_LEVEL).max(0);
                *slot = if y < WATER_LEVEL - 1 {
                    if depth <= 0 {
                        Block::Stone
                    } else {
                        Block::OrangeTerracotta
                    }
                } else if forest && y > FOREST_Y + depth * 2 {
                    if striped {
                        Block::CoarseDirt
                    } else {
                        Block::Grass
                    }
                } else if y > WATER_LEVEL + 3 + depth {
                    if !(64..=127).contains(&y) {
                        Block::OrangeTerracotta
                    } else if striped {
                        Block::Terracotta
                    } else {
                        self.band(x, y)
                    }
                } else {
                    sand_cap = true;
                    Block::RedSand
                };
            } else if left > 0 {
                left -= 1;
                *slot = if sand_cap {
                    Block::OrangeTerracotta
                } else {
                    self.band(x, y)
                };
            }
        }
    }
}

#[cfg(test)]
mod tests;
