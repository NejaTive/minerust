use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};
use glam::IVec3;

use crate::block::Block;
use crate::caves::Caves;
use crate::column::{Column, WORLD_HEIGHT};
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;
use crate::trees::hash;
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
const TAIGA_AMPLITUDE: f32 = 9.0;
const HILLS_AMPLITUDE: f32 = 34.0;

const TAIGA_COLD: (f32, f32) = (0.55, 0.7);
const BEDROCK_SCATTER: i32 = 3;
const SALT_BEDROCK: u64 = 30;
pub const FLAT_SURFACE: i32 = 4;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum WorldType {
    #[default]
    Standard,
    Flat,
}

impl WorldType {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "standard" => Some(Self::Standard),
            "flat" => Some(Self::Flat),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Biome {
    Plains,
    Desert,
    Forest,
    Taiga,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BiomeWeights {
    pub plains: f32,
    pub desert: f32,
    pub forest: f32,
    pub taiga: f32,
}

impl BiomeWeights {
    pub fn dominant(self) -> Biome {
        let land = self.plains.max(self.forest);
        if self.taiga > land.max(self.desert) {
            Biome::Taiga
        } else if self.desert > land {
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
    world_type: WorldType,
    height: FastNoiseLite,
    hilliness: FastNoiseLite,
    temperature: FastNoiseLite,
    humidity: FastNoiseLite,
    pub(crate) caves: Caves,
}

impl Terrain {
    pub fn new(seed: i32) -> Self {
        Self::of(seed, WorldType::Standard)
    }

    pub fn of(seed: i32, world_type: WorldType) -> Self {
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
            world_type,
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

    pub fn world_type(&self) -> WorldType {
        self.world_type
    }

    pub fn biome_weights(&self, x: i32, z: i32) -> BiomeWeights {
        if self.world_type == WorldType::Flat {
            return BiomeWeights {
                plains: 1.0,
                desert: 0.0,
                forest: 0.0,
                taiga: 0.0,
            };
        }
        let (x, z) = (x as f32, z as f32);
        let temperature = self.temperature.get_noise_2d(x, z);
        let humidity = self.humidity.get_noise_2d(x, z);
        let desert = smoothstep(0.05, 0.25, temperature) * smoothstep(0.1, 0.3, -humidity);
        let taiga = smoothstep(-TAIGA_COLD.0, -TAIGA_COLD.1, temperature);
        let forest = smoothstep(0.1, 0.3, humidity) * (1.0 - taiga);
        BiomeWeights {
            plains: (1.0 - desert - forest - taiga).max(0.0),
            desert,
            forest,
            taiga,
        }
    }

    pub fn biome_at(&self, x: i32, z: i32) -> Biome {
        self.biome_weights(x, z).dominant()
    }

    pub fn surface_height(&self, x: i32, z: i32) -> i32 {
        self.surface(x, z, self.biome_weights(x, z))
    }

    pub(crate) fn surface(&self, x: i32, z: i32, w: BiomeWeights) -> i32 {
        if self.world_type == WorldType::Flat {
            return FLAT_SURFACE;
        }
        let (x, z) = (x as f32, z as f32);
        let hills = smoothstep(-0.2, 0.4, self.hilliness.get_noise_2d(x, z));
        let amplitude = w.plains * PLAINS_AMPLITUDE
            + w.desert * DESERT_AMPLITUDE
            + w.forest * FOREST_AMPLITUDE
            + w.taiga * TAIGA_AMPLITUDE
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
        if self.world_type == WorldType::Flat {
            return column;
        }
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
        if self.world_type == WorldType::Flat {
            return Self::generate_flat();
        }
        let mut column = Column::new();
        let origin = key.origin();
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let (wx, wz) = (origin.x + x, origin.z + z);
                let weights = self.biome_weights(wx, wz);
                let surface = self.surface(wx, wz, weights);
                let biome = weights.dominant();
                let desert = biome == Biome::Desert;
                let taiga = biome == Biome::Taiga;
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
                } else if taiga {
                    Block::SnowyGrass
                } else {
                    Block::Grass
                };
                for y in 0..=surface {
                    if self.is_bedrock(wx, y, wz) {
                        column.set(IVec3::new(x, y, z), Block::Bedrock);
                        continue;
                    }
                    let block = match (desert, surface - y) {
                        (_, d) if d < sand_depth => Block::Sand,
                        (false, 0) => top,
                        (false, d) if d <= DIRT_DEPTH => Block::Dirt,
                        _ => Block::Stone,
                    };
                    column.set(IVec3::new(x, y, z), block);
                }
                for y in surface + 1..=WATER_LEVEL {
                    let water = if taiga && y == WATER_LEVEL {
                        Block::Ice
                    } else {
                        Block::Water
                    };
                    column.set(IVec3::new(x, y, z), water);
                }
                if column.get(IVec3::new(x, surface, z)) == Block::SnowyGrass {
                    column.set(IVec3::new(x, surface + 1, z), Block::Snow);
                }
            }
        }
        column
    }
}

impl Terrain {
    fn is_bedrock(&self, x: i32, y: i32, z: i32) -> bool {
        match y {
            0 => true,
            1..=BEDROCK_SCATTER => {
                hash(self.seed, x, z, SALT_BEDROCK + y as u64) % 4 < (4 - y) as u32
            }
            _ => false,
        }
    }

    fn generate_flat() -> Column {
        let mut column = Column::new();
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                for y in 0..=FLAT_SURFACE {
                    let block = match y {
                        0 => Block::Bedrock,
                        1 => Block::Stone,
                        FLAT_SURFACE => Block::Grass,
                        _ => Block::Dirt,
                    };
                    column.set(IVec3::new(x, y, z), block);
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
