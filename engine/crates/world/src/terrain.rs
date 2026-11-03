use fastnoise_lite::{DomainWarpType, FastNoiseLite, FractalType, NoiseType};
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
const BASE_AMPLITUDE: f32 = 8.0;
const HILLS_AMPLITUDE: f32 = 34.0;
const CONTINENT_AMPLITUDE: f32 = 4.0;
const BIOME_AMPLITUDE: [f32; 5] = [-2.0, -4.0, 4.0, 1.0, -2.0];

const CLIMATE_FREQUENCY: f32 = 0.00033;
const HUMIDITY_FREQUENCY: f32 = 0.0013;
const WARP_FREQUENCY: f32 = 0.001;
const WARP_AMPLITUDE: f32 = 160.0;
const JITTER_FREQUENCY: f32 = 0.04;
const JITTER_AMPLITUDE: f32 = 4.0;
const TEMPERATURE_BLEND: f32 = 0.018;
const HUMIDITY_BLEND: f32 = 0.06;
const TEMPERATURE_CUTS: [f32; 4] = [-0.5, -0.36, -0.048, 0.26];
const HUMIDITY_CUTS: [f32; 2] = [-0.209, 0.209];
const BIOME_TABLE: [[Biome; 3]; 5] = [
    [Biome::Tundra, Biome::Tundra, Biome::Tundra],
    [Biome::Tundra, Biome::Taiga, Biome::Taiga],
    [Biome::Plains, Biome::Plains, Biome::Forest],
    [Biome::Plains, Biome::Forest, Biome::Forest],
    [Biome::Desert, Biome::Desert, Biome::Plains],
];

const BEDROCK_SCATTER: i32 = 3;
const SALT_BEDROCK: u64 = 30;
pub const FLAT_SURFACE: i32 = 4;
pub const DEBUG_GRID_Y: i32 = 70;
pub const DEBUG_FLOOR_Y: i32 = 60;
pub const DEBUG_SPAWN_Y: i32 = 72;
const DEBUG_STEP: i32 = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum WorldType {
    #[default]
    Standard,
    Flat,
    Debug,
}

impl WorldType {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "standard" => Some(Self::Standard),
            "flat" => Some(Self::Flat),
            "debug" => Some(Self::Debug),
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
    Tundra,
}

impl Biome {
    pub fn is_snowy(self) -> bool {
        matches!(self, Biome::Taiga | Biome::Tundra)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BiomeWeights {
    pub plains: f32,
    pub desert: f32,
    pub forest: f32,
    pub taiga: f32,
    pub tundra: f32,
}

impl BiomeWeights {
    pub fn dominant(self) -> Biome {
        let all = self.as_array();
        let top = all.iter().copied().fold(0.0, f32::max);
        let index = all.iter().position(|&w| w == top).unwrap_or(0);
        BIOMES[index]
    }

    fn as_array(self) -> [f32; 5] {
        [
            self.plains,
            self.desert,
            self.forest,
            self.taiga,
            self.tundra,
        ]
    }
}

const BIOMES: [Biome; 5] = [
    Biome::Plains,
    Biome::Desert,
    Biome::Forest,
    Biome::Taiga,
    Biome::Tundra,
];

pub struct Terrain {
    seed: i32,
    world_type: WorldType,
    height: FastNoiseLite,
    erosion: FastNoiseLite,
    continentalness: FastNoiseLite,
    temperature: FastNoiseLite,
    humidity: FastNoiseLite,
    warp: FastNoiseLite,
    jitter: FastNoiseLite,
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

        let layer = |offset: i32, frequency: f32, octaves: i32| {
            let mut noise = FastNoiseLite::with_seed(seed.wrapping_add(offset));
            noise.set_noise_type(Some(NoiseType::OpenSimplex2));
            noise.set_fractal_type(Some(FractalType::FBm));
            noise.set_fractal_octaves(Some(octaves));
            noise.set_frequency(Some(frequency));
            noise
        };
        let warp = |offset: i32, frequency: f32, amplitude: f32| {
            let mut noise = FastNoiseLite::with_seed(seed.wrapping_add(offset));
            noise.set_domain_warp_type(Some(DomainWarpType::OpenSimplex2));
            noise.set_domain_warp_amp(Some(amplitude));
            noise.set_frequency(Some(frequency));
            noise
        };

        Self {
            seed,
            world_type,
            height,
            erosion: layer(1, 0.0008, 2),
            continentalness: layer(4, 0.0005, 2),
            temperature: layer(2, CLIMATE_FREQUENCY, 4),
            humidity: layer(3, HUMIDITY_FREQUENCY, 2),
            warp: warp(5, WARP_FREQUENCY, WARP_AMPLITUDE),
            jitter: warp(6, JITTER_FREQUENCY, JITTER_AMPLITUDE),
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
        if self.world_type != WorldType::Standard {
            return BiomeWeights {
                plains: 1.0,
                desert: 0.0,
                forest: 0.0,
                taiga: 0.0,
                tundra: 0.0,
            };
        }
        let (temperature, humidity) = self.climate(x, z);
        let temperature: [f32; 5] = bands(temperature, &TEMPERATURE_CUTS, TEMPERATURE_BLEND);
        let humidity: [f32; 3] = bands(humidity, &HUMIDITY_CUTS, HUMIDITY_BLEND);
        let mut weights = [0.0; 5];
        for (row, &t) in BIOME_TABLE.iter().zip(&temperature) {
            for (&biome, &h) in row.iter().zip(&humidity) {
                weights[biome as usize] += t * h;
            }
        }
        BiomeWeights {
            plains: weights[Biome::Plains as usize],
            desert: weights[Biome::Desert as usize],
            forest: weights[Biome::Forest as usize],
            taiga: weights[Biome::Taiga as usize],
            tundra: weights[Biome::Tundra as usize],
        }
    }

    pub(crate) fn climate(&self, x: i32, z: i32) -> (f32, f32) {
        let (x, z) = self.warp.domain_warp_2d(x as f32, z as f32);
        let (x, z) = self.jitter.domain_warp_2d(x, z);
        (
            self.temperature.get_noise_2d(x, z),
            self.humidity.get_noise_2d(x, z),
        )
    }

    /// Веса биомов колонки для `Renderer::set_biome_column`: строки по z, внутри x, четыре
    /// байта на блок: равнины, пустыня, лес, холод (тайга + тундра).
    pub fn column_biome_bytes(&self, key: ColumnPos) -> Vec<u8> {
        let origin = key.origin();
        let mut out = Vec::with_capacity((SECTION_SIZE * SECTION_SIZE * 4) as usize);
        for dz in 0..SECTION_SIZE {
            for dx in 0..SECTION_SIZE {
                let weights = self.biome_weights(origin.x + dx, origin.z + dz);
                out.extend(crate::biome_color::weights_bytes(weights));
            }
        }
        out
    }

    pub fn biome_at(&self, x: i32, z: i32) -> Biome {
        self.biome_weights(x, z).dominant()
    }

    pub fn surface_height(&self, x: i32, z: i32) -> i32 {
        self.surface(x, z, self.biome_weights(x, z))
    }

    pub(crate) fn surface(&self, x: i32, z: i32, w: BiomeWeights) -> i32 {
        match self.world_type {
            WorldType::Flat => return FLAT_SURFACE,
            WorldType::Debug => return DEBUG_GRID_Y,
            WorldType::Standard => {}
        }
        let (x, z) = (x as f32, z as f32);
        let hills = smoothstep(-0.2, 0.4, self.erosion.get_noise_2d(x, z));
        let biome: f32 = w
            .as_array()
            .iter()
            .zip(BIOME_AMPLITUDE)
            .map(|(weight, delta)| weight * delta)
            .sum();
        let amplitude = BASE_AMPLITUDE + biome + HILLS_AMPLITUDE * hills;
        let y = SEA_LEVEL
            + CONTINENT_AMPLITUDE * self.continentalness.get_noise_2d(x, z)
            + amplitude * self.height.get_noise_2d(x, z);
        (y.floor() as i32).clamp(1, WORLD_HEIGHT - 2)
    }

    pub fn generate(&self, key: ColumnPos) -> Column {
        if self.world_type == WorldType::Debug {
            return Self::generate_debug(key);
        }
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
        if self.world_type != WorldType::Standard {
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
        match self.world_type {
            WorldType::Flat => return Self::generate_flat(),
            WorldType::Debug => return Self::generate_debug(key),
            WorldType::Standard => {}
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
                let snowy = biome.is_snowy();
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
                } else if snowy {
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
                    let water = if snowy && y == WATER_LEVEL {
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

    fn generate_debug(key: ColumnPos) -> Column {
        let mut column = Column::new();
        let origin = key.origin();
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                column.set(IVec3::new(x, DEBUG_FLOOR_Y, z), Block::Barrier);
            }
        }
        let states = debug_states();
        let side = debug_grid_side(states.len());
        for (cell, (block, data)) in states.into_iter().enumerate() {
            let (wx, wz) = debug_cell_origin(cell, side);
            let (x, z) = (wx - origin.x, wz - origin.z);
            if (0..SECTION_SIZE).contains(&x) && (0..SECTION_SIZE).contains(&z) {
                column.set_with_data(IVec3::new(x, DEBUG_GRID_Y, z), block, data);
            }
        }
        column
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

pub fn debug_states() -> Vec<(Block, u8)> {
    Block::ALL
        .into_iter()
        .skip(1)
        .flat_map(|block| block.states().map(move |data| (block, data)))
        .collect()
}

pub fn debug_grid_side(cells: usize) -> i32 {
    (cells as f64).sqrt().ceil() as i32
}

fn debug_cell_origin(cell: usize, side: i32) -> (i32, i32) {
    let cell = cell as i32;
    (cell % side * DEBUG_STEP, cell / side * DEBUG_STEP)
}

fn bands<const N: usize, const M: usize>(value: f32, cuts: &[f32; N], blend: f32) -> [f32; M] {
    let mut out = [0.0; M];
    let mut above = 1.0;
    for (slot, &cut) in out.iter_mut().zip(cuts) {
        let step = smoothstep(cut - blend, cut + blend, value);
        *slot = above - step;
        above = step;
    }
    out[N] = above;
    out
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
