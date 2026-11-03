use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};
use glam::IVec3;

use crate::biome_color::{BLEND_RADIUS, BiomePalette};
use crate::biome_map::BiomeMap;
use crate::block::Block;
use crate::caves::Caves;
use crate::column::{Column, WORLD_HEIGHT};
use crate::mesa::{LAYER_SPAN, Mesa};
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;
use crate::trees::hash;
use crate::{plants, trees};

pub use crate::biome::Biome;

const DIRT_DEPTH: i32 = 3;
pub const WATER_LEVEL: i32 = 62;
const GRAVEL_DEPTH: i32 = 2;
const SEA_BASE: f32 = 63.0;
const HEIGHT_PER_BASE: f32 = 17.0;
const SPREAD_AMPLITUDE: f32 = 50.0;
const WINDOW_RADIUS: i32 = 2;
const GRID: usize = 5;
const GRID_STEP: i32 = 4;
const COARSE_SIDE: usize = GRID + 2 * WINDOW_RADIUS as usize;
const CLIMATE_CACHE_COLUMNS: usize = 1024;
const MOUNTAIN_STONE_Y: i32 = 95;
const MOUNTAIN_GRAVEL_Y: i32 = 80;

const BEDROCK_SCATTER: i32 = 3;
const SALT_BEDROCK: u64 = 30;
const SALT_SCREE: u64 = 31;
const SALT_SANDSTONE: u64 = 32;
const SWAMP_NOISE_SALT: i32 = 2345;
const SWAMP_NOISE_FREQUENCY: f32 = 0.25;
const SWAMP_LILY_BELOW: f32 = 0.12;
const SANDSTONE_JITTER: u32 = 4;
const SANDSTONE_BASE_Y: i32 = 63;
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

struct ColumnClimate {
    biomes: [Biome; 256],
    surface: [i32; 256],
}

impl ColumnClimate {
    fn index(x: i32, z: i32) -> usize {
        ((z & 15) * SECTION_SIZE + (x & 15)) as usize
    }
}

pub struct Terrain {
    seed: i32,
    world_type: WorldType,
    height: FastNoiseLite,
    map: BiomeMap,
    mesa: Mesa,
    swamp: FastNoiseLite,
    climate: Mutex<HashMap<(i32, i32), Arc<ColumnClimate>>>,
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

        let mut swamp = FastNoiseLite::with_seed(seed ^ SWAMP_NOISE_SALT);
        swamp.set_noise_type(Some(NoiseType::Perlin));
        swamp.set_frequency(Some(SWAMP_NOISE_FREQUENCY));

        Self {
            seed,
            world_type,
            height,
            swamp,
            map: BiomeMap::new(seed),
            mesa: Mesa::new(seed),
            climate: Mutex::new(HashMap::new()),
            caves: Caves::new(seed),
        }
    }

    pub fn seed(&self) -> i32 {
        self.seed
    }

    pub fn world_type(&self) -> WorldType {
        self.world_type
    }

    pub fn biome_at(&self, x: i32, z: i32) -> Biome {
        if self.world_type != WorldType::Standard {
            return Biome::Plains;
        }
        self.climate(x >> 4, z >> 4).biomes[ColumnClimate::index(x, z)]
    }

    pub fn sample_biome(&self, x: i32, z: i32) -> Biome {
        if self.world_type != WorldType::Standard {
            return Biome::Plains;
        }
        self.map.blocks(x, z, 1, 1)[0]
    }

    fn swamp_noise(&self, x: i32, z: i32) -> f32 {
        self.swamp.get_noise_2d(x as f32, z as f32)
    }

    /// Лужа болота: верхний блок на уровне воды заменён водой (`BiomeSwamp.genTerrainBlocks`).
    pub(crate) fn is_puddle(&self, x: i32, z: i32) -> bool {
        self.world_type == WorldType::Standard
            && self.biome_at(x, z).is_swamp()
            && self.surface_height(x, z) == WATER_LEVEL
            && self.swamp_noise(x, z) > 0.0
    }

    pub fn surface_height(&self, x: i32, z: i32) -> i32 {
        match self.world_type {
            WorldType::Flat => FLAT_SURFACE,
            WorldType::Debug => DEBUG_GRID_Y,
            WorldType::Standard => self.climate(x >> 4, z >> 4).surface[ColumnClimate::index(x, z)],
        }
    }

    /// Цвета травы и листвы колонки для `Renderer::set_biome_column`, см. `BiomePalette::column_bytes`.
    pub fn column_tint_bytes(&self, key: ColumnPos, palette: &BiomePalette) -> Vec<u8> {
        let side = SECTION_SIZE as usize + 2 * BLEND_RADIUS as usize;
        let biomes = if self.world_type == WorldType::Standard {
            let origin = key.origin();
            self.map
                .blocks(origin.x - BLEND_RADIUS, origin.z - BLEND_RADIUS, side, side)
        } else {
            vec![Biome::Plains; side * side]
        };
        palette.column_bytes(&biomes)
    }

    fn climate(&self, column_x: i32, column_z: i32) -> Arc<ColumnClimate> {
        let mut cache = self.climate.lock().expect("climate cache poisoned");
        if let Some(found) = cache.get(&(column_x, column_z)) {
            return Arc::clone(found);
        }
        if cache.len() >= CLIMATE_CACHE_COLUMNS {
            cache.clear();
        }
        let computed = Arc::new(self.compute_climate(column_x, column_z));
        cache.insert((column_x, column_z), Arc::clone(&computed));
        computed
    }

    fn compute_climate(&self, column_x: i32, column_z: i32) -> ColumnClimate {
        let (x0, z0) = (column_x * SECTION_SIZE, column_z * SECTION_SIZE);
        let margin = WINDOW_RADIUS;
        let coarse = self.map.coarse(
            x0 / GRID_STEP - margin,
            z0 / GRID_STEP - margin,
            COARSE_SIDE,
            COARSE_SIDE,
        );
        let cell = |i: usize, j: usize| {
            Biome::from_id(coarse.data[j * COARSE_SIDE + i]).unwrap_or(Biome::Plains)
        };
        let mut base = [[0.0f32; GRID]; GRID];
        let mut spread = [[0.0f32; GRID]; GRID];
        for (gz, (base_row, spread_row)) in base.iter_mut().zip(&mut spread).enumerate() {
            for (gx, (b, v)) in base_row.iter_mut().zip(spread_row).enumerate() {
                (*b, *v) = blended_heights(&cell, gx, gz);
            }
        }
        let blocks = self.map.blocks_within(x0, z0, 16, 16, &coarse);
        let mut biomes = [Biome::Plains; 256];
        let mut surface = [0; 256];
        let step = GRID_STEP as usize;
        for z in 0..16usize {
            for x in 0..16usize {
                let (gx, gz) = (x / step, z / step);
                let (fx, fz) = (
                    (x % step) as f32 / step as f32,
                    (z % step) as f32 / step as f32,
                );
                let lerp = |grid: &[[f32; GRID]; GRID]| {
                    let top = grid[gz][gx] * (1.0 - fx) + grid[gz][gx + 1] * fx;
                    let bottom = grid[gz + 1][gx] * (1.0 - fx) + grid[gz + 1][gx + 1] * fx;
                    top * (1.0 - fz) + bottom * fz
                };
                let noise = self
                    .height
                    .get_noise_2d((x0 + x as i32) as f32, (z0 + z as i32) as f32);
                let y = SEA_BASE
                    + HEIGHT_PER_BASE * lerp(&base)
                    + SPREAD_AMPLITUDE * lerp(&spread) * noise;
                let index = z * 16 + x;
                biomes[index] = blocks[index];
                let mut height = (y.floor() as i32).clamp(1, WORLD_HEIGHT - 2);
                if blocks[index] == Biome::MesaBryce {
                    height = height.max(self.mesa.pillar_top(x0 + x as i32, z0 + z as i32));
                }
                surface[index] = height;
            }
        }
        ColumnClimate { biomes, surface }
    }

    pub fn generate(&self, key: ColumnPos) -> Column {
        if self.world_type == WorldType::Debug {
            return Self::generate_debug(key);
        }
        let mut column = self.generate_trees(key);
        plants::plant(self, key, &mut column);
        self.decorate(key, &mut column);
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
        let climate = self.climate(key.x(), key.z());
        for z in 0..SECTION_SIZE {
            for x in 0..SECTION_SIZE {
                let (wx, wz) = (origin.x + x, origin.z + z);
                let index = ColumnClimate::index(x, z);
                let surface = climate.surface[index];
                let biome = climate.biomes[index];
                let snowy = biome.is_snowy();
                let (top, filler) = self.surface_blocks(biome, surface, wx, wz);
                let mut mesa_layers = [Block::Stone; LAYER_SPAN];
                let mesa = biome.is_mesa() && surface >= WATER_LEVEL - 1;
                if mesa {
                    self.mesa.layers(biome, surface, wx, wz, &mut mesa_layers);
                }
                let sandstone = if filler == Block::Sand {
                    (hash(self.seed, wx, wz, SALT_SANDSTONE) % SANDSTONE_JITTER) as i32
                        + (surface - SANDSTONE_BASE_Y).max(0)
                } else {
                    0
                };
                for y in 0..=surface {
                    if self.is_bedrock(wx, y, wz) {
                        column.set(IVec3::new(x, y, z), Block::Bedrock);
                        continue;
                    }
                    let depth = surface - y;
                    let block = match depth {
                        _ if mesa => mesa_layers
                            .get(depth as usize)
                            .copied()
                            .unwrap_or(Block::Stone),
                        0 => top,
                        d if d <= DIRT_DEPTH => filler,
                        d if d <= DIRT_DEPTH + sandstone => Block::Sandstone,
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
                if biome.is_swamp() && surface == WATER_LEVEL {
                    let noise = self.swamp_noise(wx, wz);
                    if noise > 0.0 {
                        column.set(IVec3::new(x, surface, z), Block::Water);
                        if noise < SWAMP_LILY_BELOW {
                            column.set(IVec3::new(x, surface + 1, z), Block::Waterlily);
                        }
                    }
                }
                if column.get(IVec3::new(x, surface, z)) == Block::SnowyGrass {
                    column.set(IVec3::new(x, surface + 1, z), Block::Snow);
                }
            }
        }
        column
    }

    fn surface_blocks(&self, biome: Biome, surface: i32, x: i32, z: i32) -> (Block, Block) {
        let blocks = biome.surface_blocks();
        let depth = WATER_LEVEL - surface;
        if depth > GRAVEL_DEPTH {
            return (Block::Gravel, blocks.filler);
        }
        if depth > 0 {
            let shallow = if biome.is_ocean() || biome.is_river() {
                Block::Sand
            } else if matches!(blocks.top, Block::Grass | Block::Mycelium) {
                Block::Dirt
            } else {
                blocks.top
            };
            return (shallow, blocks.filler);
        }
        if biome.is_extreme_hills() {
            if surface > MOUNTAIN_STONE_Y {
                return (Block::Stone, Block::Stone);
            }
            let scree = biome.is_mutation()
                && surface > MOUNTAIN_GRAVEL_Y
                && hash(self.seed, x, z, SALT_SCREE).is_multiple_of(4);
            if scree {
                return (Block::Gravel, Block::Stone);
            }
        }
        if blocks.top == Block::Grass && biome.is_snowy() {
            return (Block::SnowyGrass, blocks.filler);
        }
        (blocks.top, blocks.filler)
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

fn blended_heights(cell: &impl Fn(usize, usize) -> Biome, gx: usize, gz: usize) -> (f32, f32) {
    let center = cell(gx + WINDOW_RADIUS as usize, gz + WINDOW_RADIUS as usize);
    let (mut base, mut spread, mut total) = (0.0, 0.0, 0.0);
    for dz in -WINDOW_RADIUS..=WINDOW_RADIUS {
        for dx in -WINDOW_RADIUS..=WINDOW_RADIUS {
            let near = cell(
                (gx as i32 + WINDOW_RADIUS + dx) as usize,
                (gz as i32 + WINDOW_RADIUS + dz) as usize,
            );
            let distance = (dx * dx + dz * dz) as f32 + 0.2;
            let mut weight = 10.0 / distance.sqrt() / (near.base_height() + 2.0);
            if near.base_height() > center.base_height() {
                weight /= 2.0;
            }
            base += near.base_height() * weight;
            spread += near.height_variation() * weight;
            total += weight;
        }
    }
    (base / total, spread / total * 0.9 + 0.1)
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

mod decor;

#[cfg(test)]
mod bench;
#[cfg(test)]
mod tests;
