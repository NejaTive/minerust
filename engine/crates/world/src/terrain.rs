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

use self::relief::{Relief, Solid};

pub use crate::biome::Biome;

pub const WATER_LEVEL: i32 = 62;
const SEA_LEVEL: i32 = WATER_LEVEL + 1;
const WINDOW_RADIUS: i32 = 2;
const GRID: usize = 5;
const GRID_STEP: i32 = 4;
const COARSE_SIDE: usize = GRID + 2 * WINDOW_RADIUS as usize;
const CLIMATE_CACHE_COLUMNS: usize = 2048;
const FREEZE_BELOW: f32 = 0.15;
const GRAVEL_BELOW_SEA: i32 = 7;
const HILLS_STONE_ABOVE: f64 = 1.0;
const HILLS_GRAVEL_ABOVE: f64 = 2.0;
const HILLS_GRAVEL_BELOW: f64 = -1.0;
const TOP_THICKNESS_BASE: f64 = 3.0;
const TOP_THICKNESS_JITTER: f64 = 0.25;

const BEDROCK_SCATTER: i32 = 4;
const BEDROCK_ODDS: u32 = 5;
const SALT_BEDROCK: u64 = 30;
const SALT_THICKNESS: u64 = 33;
const SALT_SANDSTONE: u64 = 32;
const SWAMP_NOISE_SALT: i32 = 2345;
const MEGA_NOISE_SALT: i32 = 3456;
const COLOR_NOISE_SALT: i32 = 4567;
const MEGA_NOISE_FREQUENCY: f32 = 0.0625;
const MEGA_NOISE_SCALE: f32 = 4.0;
const MEGA_COARSE_ABOVE: f32 = 1.75;
const MEGA_PODZOL_ABOVE: f32 = -0.95;
const SWAMP_NOISE_FREQUENCY: f32 = 0.25;
const SWAMP_LILY_BELOW: f32 = 0.12;
const SANDSTONE_JITTER: u32 = 4;
const SANDSTONE_BASE_Y: i32 = SEA_LEVEL;
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
    solid: [Solid; 256],
}

impl ColumnClimate {
    fn index(x: i32, z: i32) -> usize {
        ((z & 15) * SECTION_SIZE + (x & 15)) as usize
    }
}

type LakeCache = HashMap<(i32, i32), Arc<Vec<decor::lakes::Lake>>>;

pub struct Terrain {
    seed: i32,
    world_type: WorldType,
    relief: Relief,
    map: BiomeMap,
    mesa: Mesa,
    swamp: FastNoiseLite,
    mega: FastNoiseLite,
    color: FastNoiseLite,
    climate: Mutex<HashMap<(i32, i32), Arc<ColumnClimate>>>,
    lakes: Mutex<LakeCache>,
    pub(crate) caves: Caves,
}

impl Terrain {
    pub fn new(seed: i32) -> Self {
        Self::of(seed, WorldType::Standard)
    }

    pub fn of(seed: i32, world_type: WorldType) -> Self {
        let mut swamp = FastNoiseLite::with_seed(seed ^ SWAMP_NOISE_SALT);
        swamp.set_noise_type(Some(NoiseType::Perlin));
        swamp.set_frequency(Some(SWAMP_NOISE_FREQUENCY));

        let mut mega = FastNoiseLite::with_seed(seed ^ MEGA_NOISE_SALT);
        mega.set_noise_type(Some(NoiseType::OpenSimplex2));
        mega.set_fractal_type(Some(FractalType::FBm));
        mega.set_fractal_octaves(Some(4));
        mega.set_frequency(Some(MEGA_NOISE_FREQUENCY));

        let mut color = FastNoiseLite::with_seed(seed ^ COLOR_NOISE_SALT);
        color.set_noise_type(Some(NoiseType::Perlin));
        color.set_frequency(Some(1.0));

        Self {
            seed,
            world_type,
            relief: Relief::new(seed),
            swamp,
            mega,
            color,
            map: BiomeMap::new(seed),
            mesa: Mesa::new(seed),
            climate: Mutex::new(HashMap::new()),
            lakes: Mutex::new(HashMap::new()),
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

    pub(crate) fn color_noise(&self, x: i32, z: i32, scale: f32) -> f32 {
        self.color.get_noise_2d(x as f32 / scale, z as f32 / scale)
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
        let blocks = self.map.blocks_within(x0, z0, 16, 16, &coarse);
        let mut solid = self.relief.solid(column_x, column_z, &cell);
        let mut biomes = [Biome::Plains; 256];
        let mut surface = [0; 256];
        for z in 0..16usize {
            for x in 0..16usize {
                let index = z * 16 + x;
                biomes[index] = blocks[index];
                if blocks[index] == Biome::MesaBryce {
                    let pillar = self.mesa.pillar_top(x0 + x as i32, z0 + z as i32);
                    for y in (0..=pillar.min(WORLD_HEIGHT - 1)).rev() {
                        if solid[index].get(y) {
                            break;
                        }
                        solid[index].set(y);
                    }
                }
                surface[index] = solid[index].highest().unwrap_or(1).max(1);
            }
        }
        ColumnClimate {
            biomes,
            surface,
            solid,
        }
    }

    pub fn generate(&self, key: ColumnPos) -> Column {
        if self.world_type == WorldType::Debug {
            return Self::generate_debug(key);
        }
        let mut column = self.generate_carved(key);
        self.grow_lakes(key, &mut column);
        self.grow_ores(key, &mut column);
        self.grow_fossils(key, &mut column);
        trees::plant_where(self, key, &mut column, |tree| {
            !self.lake_covers(tree.base.x, tree.base.z)
        });
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
                let biome = climate.biomes[index];
                let mut blocks: [Block; WORLD_HEIGHT as usize] = std::array::from_fn(|y| {
                    if climate.solid[index].get(y as i32) {
                        Block::Stone
                    } else if y as i32 <= WATER_LEVEL {
                        Block::Water
                    } else {
                        Block::Air
                    }
                });
                if biome.is_swamp() && climate.surface[index] == WATER_LEVEL {
                    let noise = self.swamp_noise(wx, wz);
                    if noise > 0.0 {
                        blocks[WATER_LEVEL as usize] = Block::Water;
                        if noise < SWAMP_LILY_BELOW {
                            blocks[WATER_LEVEL as usize + 1] = Block::Waterlily;
                        }
                    }
                }
                self.paint_column(&mut blocks, biome, wx, wz);
                self.freeze_top(&mut blocks, biome, wx, wz);
                let top = blocks
                    .iter()
                    .rposition(|&b| b != Block::Air)
                    .map_or(0, |y| y + 1);
                column.set_stack(x, z, &blocks[..top]);
            }
        }
        column
    }

    fn temperature(&self, biome: Biome, x: i32, y: i32, z: i32) -> f32 {
        self.relief.temperature(biome, x, y, z)
    }

    /// Порт `Biome.generateBiomeTerrain`: слои сверху вниз, после каждого воздушного промежутка заново.
    fn paint_column(
        &self,
        blocks: &mut [Block; WORLD_HEIGHT as usize],
        biome: Biome,
        x: i32,
        z: i32,
    ) {
        let noise = self.relief.surface_noise(x, z);
        if biome.is_mesa() {
            self.paint_mesa(blocks, biome, x, z);
        } else {
            self.paint_layers(blocks, biome, x, z, noise);
        }
        for y in 0..=BEDROCK_SCATTER {
            if self.is_bedrock(x, y, z) {
                blocks[y as usize] = Block::Bedrock;
            }
        }
    }

    fn paint_mesa(
        &self,
        blocks: &mut [Block; WORLD_HEIGHT as usize],
        biome: Biome,
        x: i32,
        z: i32,
    ) {
        let mut y = WORLD_HEIGHT - 1;
        while y >= 0 {
            if blocks[y as usize] != Block::Stone {
                y -= 1;
                continue;
            }
            let mut layers = [Block::Stone; LAYER_SPAN];
            self.mesa.layers(biome, y, x, z, &mut layers);
            let mut depth = 0;
            while y >= 0 && blocks[y as usize] == Block::Stone {
                if let Some(&layer) = layers.get(depth) {
                    blocks[y as usize] = layer;
                }
                depth += 1;
                y -= 1;
            }
        }
    }

    fn paint_layers(
        &self,
        blocks: &mut [Block; WORLD_HEIGHT as usize],
        biome: Biome,
        x: i32,
        z: i32,
        noise: f64,
    ) {
        let base = self.surface_blocks(biome, x, z, noise);
        let (mut top, mut filler) = (base.0, base.1);
        let jitter = f64::from(hash(self.seed, x, z, SALT_THICKNESS)) / f64::from(u32::MAX);
        let thickness = (noise / 3.0 + TOP_THICKNESS_BASE + jitter * TOP_THICKNESS_JITTER) as i32;
        let mut left = -1;
        let top_y = blocks
            .iter()
            .rposition(|&b| b != Block::Air)
            .map_or(0, |y| y as i32);
        for y in (0..=top_y).rev() {
            match blocks[y as usize] {
                Block::Air => left = -1,
                Block::Stone if left == -1 => {
                    if thickness <= 0 {
                        (top, filler) = (Block::Air, Block::Stone);
                    } else if (SEA_LEVEL - 4..=SEA_LEVEL + 1).contains(&y) {
                        (top, filler) = base;
                    }
                    if y < SEA_LEVEL && top == Block::Air {
                        top = if self.temperature(biome, x, y, z) < FREEZE_BELOW {
                            Block::Ice
                        } else {
                            Block::Water
                        };
                    }
                    left = thickness;
                    if y >= SEA_LEVEL - 1 {
                        if top != Block::Air {
                            blocks[y as usize] = top;
                        }
                    } else if y < SEA_LEVEL - GRAVEL_BELOW_SEA - thickness {
                        (top, filler) = (Block::Air, Block::Stone);
                        blocks[y as usize] = Block::Gravel;
                    } else {
                        blocks[y as usize] = filler;
                    }
                }
                Block::Stone if left > 0 => {
                    left -= 1;
                    blocks[y as usize] = filler;
                    if left == 0 && filler == Block::Sand && thickness > 1 {
                        left = (hash(self.seed, x, z, SALT_SANDSTONE) % SANDSTONE_JITTER) as i32
                            + (y - SANDSTONE_BASE_Y).max(0);
                        filler = Block::Sandstone;
                    }
                }
                _ => {}
            }
        }
    }

    /// Лёд на воде и снег на верхнем блоке, где `Biome.getTemperature(pos)` ниже порога замерзания.
    fn freeze_top(
        &self,
        blocks: &mut [Block; WORLD_HEIGHT as usize],
        biome: Biome,
        x: i32,
        z: i32,
    ) {
        let Some(top) = blocks.iter().rposition(|&b| b != Block::Air) else {
            return;
        };
        let y = top as i32;
        match blocks[top] {
            Block::Water => {
                if y == WATER_LEVEL && self.temperature(biome, x, y, z) < FREEZE_BELOW {
                    blocks[top] = Block::Ice;
                }
            }
            Block::Ice | Block::Waterlily | Block::Lava | Block::Bedrock => {}
            ground => {
                if top + 1 < blocks.len() && self.temperature(biome, x, y + 1, z) < FREEZE_BELOW {
                    blocks[top + 1] = Block::Snow;
                    if ground == Block::Grass {
                        blocks[top] = Block::SnowyGrass;
                    }
                }
            }
        }
    }

    fn surface_blocks(&self, biome: Biome, x: i32, z: i32, noise: f64) -> (Block, Block) {
        let blocks = biome.surface_blocks();
        if biome.is_extreme_hills() {
            let extra_trees = biome == Biome::ExtremeHillsPlus;
            if biome.is_mutation() && !(HILLS_GRAVEL_BELOW..=HILLS_GRAVEL_ABOVE).contains(&noise) {
                return (Block::Gravel, Block::Gravel);
            }
            if noise > HILLS_STONE_ABOVE && !extra_trees {
                return (Block::Stone, Block::Stone);
            }
        }
        if biome.is_mega_taiga() {
            let noise = self.mega.get_noise_2d(x as f32, z as f32) * MEGA_NOISE_SCALE;
            let top = if noise > MEGA_COARSE_ABOVE {
                Block::CoarseDirt
            } else if noise > MEGA_PODZOL_ABOVE {
                Block::Podzol
            } else {
                Block::Grass
            };
            return (top, blocks.filler);
        }
        (blocks.top, blocks.filler)
    }
}

impl Terrain {
    fn is_bedrock(&self, x: i32, y: i32, z: i32) -> bool {
        match y {
            0 => true,
            1..=BEDROCK_SCATTER => {
                hash(self.seed, x, z, SALT_BEDROCK + y as u64) % BEDROCK_ODDS
                    < BEDROCK_ODDS - y as u32
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
mod relief;

#[cfg(test)]
mod bench;
#[cfg(test)]
mod tests;
