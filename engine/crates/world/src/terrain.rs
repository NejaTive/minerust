use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};
use glam::IVec3;

use crate::block::Block;
use crate::caves::Caves;
use crate::column::{Column, WORLD_HEIGHT};
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;
use crate::{plants, trees};

const DIRT_DEPTH: i32 = 3;
const SEA_LEVEL: f32 = 64.0;
pub const WATER_LEVEL: i32 = 62;
const GRAVEL_DEPTH: i32 = 2;
const BEACH: std::ops::RangeInclusive<i32> = WATER_LEVEL - 2..=WATER_LEVEL + 1;
const BEACH_SAND_DEPTH: i32 = 3;
const PLAINS_AMPLITUDE: f32 = 6.0;
const DESERT_AMPLITUDE: f32 = 4.0;
const FOREST_AMPLITUDE: f32 = 14.0;
const HILLS_AMPLITUDE: f32 = 34.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Biome {
    Plains,
    Desert,
    Forest,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BiomeWeights {
    pub plains: f32,
    pub desert: f32,
    pub forest: f32,
}

impl BiomeWeights {
    pub fn dominant(self) -> Biome {
        if self.desert > self.plains.max(self.forest) {
            Biome::Desert
        } else if self.forest > self.plains {
            Biome::Forest
        } else {
            Biome::Plains
        }
    }
}

pub struct Terrain {
    seed: i32,
    height: FastNoiseLite,
    hilliness: FastNoiseLite,
    temperature: FastNoiseLite,
    humidity: FastNoiseLite,
    pub(crate) caves: Caves,
}

impl Terrain {
    pub fn new(seed: i32) -> Self {
        let mut height = FastNoiseLite::with_seed(seed);
        height.set_noise_type(Some(NoiseType::OpenSimplex2));
        height.set_fractal_type(Some(FractalType::FBm));
        height.set_fractal_octaves(Some(5));
        height.set_frequency(Some(0.005));

        let mut hilliness = FastNoiseLite::with_seed(seed.wrapping_add(1));
        hilliness.set_noise_type(Some(NoiseType::OpenSimplex2));
        hilliness.set_frequency(Some(0.0015));

        let climate = |offset| {
            let mut noise = FastNoiseLite::with_seed(seed.wrapping_add(offset));
            noise.set_noise_type(Some(NoiseType::OpenSimplex2));
            noise.set_frequency(Some(0.003));
            noise
        };

        Self {
            seed,
            height,
            hilliness,
            temperature: climate(2),
            humidity: climate(3),
            caves: Caves::new(seed),
        }
    }

    pub fn seed(&self) -> i32 {
        self.seed
    }

    pub fn biome_weights(&self, x: i32, z: i32) -> BiomeWeights {
        let (x, z) = (x as f32, z as f32);
        let temperature = self.temperature.get_noise_2d(x, z);
        let humidity = self.humidity.get_noise_2d(x, z);
        let desert = smoothstep(0.05, 0.25, temperature) * smoothstep(0.1, 0.3, -humidity);
        let forest = smoothstep(0.1, 0.3, humidity);
        BiomeWeights {
            plains: 1.0 - desert - forest,
            desert,
            forest,
        }
    }

    pub fn biome_at(&self, x: i32, z: i32) -> Biome {
        self.biome_weights(x, z).dominant()
    }

    pub fn surface_height(&self, x: i32, z: i32) -> i32 {
        self.surface(x, z, self.biome_weights(x, z))
    }

    pub(crate) fn surface(&self, x: i32, z: i32, w: BiomeWeights) -> i32 {
        let (x, z) = (x as f32, z as f32);
        let hills = smoothstep(-0.2, 0.4, self.hilliness.get_noise_2d(x, z));
        let amplitude = w.plains * PLAINS_AMPLITUDE
            + w.desert * DESERT_AMPLITUDE
            + w.forest * FOREST_AMPLITUDE
            + HILLS_AMPLITUDE * hills;
        let y = SEA_LEVEL + amplitude * self.height.get_noise_2d(x, z);
        (y.floor() as i32).clamp(1, WORLD_HEIGHT - 2)
    }

    pub fn generate(&self, key: ColumnPos) -> Column {
        let mut column = self.generate_trees(key);
        plants::plant(self, key, &mut column);
        column
    }

    /// Колонка с пещерами и деревьями, без растений: тесты мешера и деревьев сверяют кубы.
    #[doc(hidden)]
    pub fn generate_trees(&self, key: ColumnPos) -> Column {
        let mut column = self.generate_carved(key);
        trees::plant(self, key, &mut column);
        column
    }

    #[doc(hidden)]
    pub fn generate_carved(&self, key: ColumnPos) -> Column {
        let mut column = self.generate_ground(key);
        self.caves.carve(
            key,
            &mut column,
            |x, z| self.surface_height(x, z),
            |x, z| trees::tree_at(self, x, z).is_some(),
        );
        column
    }

    #[doc(hidden)]
    pub fn generate_ground(&self, key: ColumnPos) -> Column {
        let mut column = Column::new();
        let origin = key.origin();
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let (wx, wz) = (origin.x + x, origin.z + z);
                let weights = self.biome_weights(wx, wz);
                let surface = self.surface(wx, wz, weights);
                let desert = weights.dominant() == Biome::Desert;
                let sand_depth = if desert {
                    (3.0 + weights.desert).round() as i32
                } else if BEACH.contains(&surface) {
                    BEACH_SAND_DEPTH
                } else {
                    0
                };
                let top = if WATER_LEVEL - surface > GRAVEL_DEPTH {
                    Block::Gravel
                } else if surface < WATER_LEVEL {
                    Block::Dirt
                } else {
                    Block::Grass
                };
                for y in 0..=surface {
                    let block = match (desert, surface - y) {
                        (_, d) if d < sand_depth => Block::Sand,
                        (false, 0) => top,
                        (false, d) if d <= DIRT_DEPTH => Block::Dirt,
                        _ => Block::Stone,
                    };
                    column.set(IVec3::new(x, y, z), block);
                }
                for y in surface + 1..=WATER_LEVEL {
                    column.set(IVec3::new(x, y, z), Block::Water);
                }
            }
        }
        column
    }
}

pub(crate) fn smoothstep(edge0: f32, edge1: f32, v: f32) -> f32 {
    let t = ((v - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[doc(hidden)]
pub fn lake_key(terrain: &Terrain) -> ColumnPos {
    let deep = |x, z| terrain.surface_height(x, z) < WATER_LEVEL - 2;
    (0..4096)
        .flat_map(|z| (-64..64).map(move |i| (i * SECTION_SIZE, z * 3)))
        .find(|&(x, z)| deep(x, z) && deep(x - 1, z))
        .map(|(x, z)| ColumnPos::new(x / SECTION_SIZE, z.div_euclid(SECTION_SIZE)))
        .expect("no lake on a column border")
}

#[cfg(test)]
mod bench;
#[cfg(test)]
mod tests;
