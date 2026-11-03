use crate::block::Block;
use crate::column::{Column, WORLD_HEIGHT};
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;
use crate::terrain::{Biome, Terrain, WATER_LEVEL, WorldType};
use crate::trees::hash;

const GRASS_DENSITY: f32 = 0.12;
const FLOWER_DENSITY: f32 = 0.01;
const MAX_GRASS_SCALE: f32 = 2.0;
const MAX_FLOWER_SCALE: f32 = 12.0;
const FLOWER_FOREST_SCALE: f32 = 12.0;
const TULIP_NOISE_SCALE: f32 = 200.0;
const TULIP_BELOW: f32 = -0.55;
const FLOWER_FOREST_NOISE_SCALE: f32 = 48.0;
const SWAMP_FLOWER_SCALE: f32 = 1.0;
const FERN_ONE_IN_JUNGLE: u32 = 4;
const FERN_IN_TAIGA: u32 = 4;
const FERN_OUT_OF_TAIGA: u32 = 5;
const FLOWER_FOREST_PICKS: [Block; 10] = [
    Block::Dandelion,
    Block::Poppy,
    Block::Poppy,
    Block::Allium,
    Block::Houstonia,
    Block::RedTulip,
    Block::OrangeTulip,
    Block::WhiteTulip,
    Block::PinkTulip,
    Block::OxeyeDaisy,
];
const PLAINS_TULIPS: [Block; 4] = [
    Block::OrangeTulip,
    Block::RedTulip,
    Block::PinkTulip,
    Block::WhiteTulip,
];
const PLAINS_DAISIES: [Block; 3] = [Block::Poppy, Block::Houstonia, Block::OxeyeDaisy];

const SALT_PLANT: u64 = 11;
const SALT_KIND: u64 = 12;
const SALT_FERN: u64 = 13;

fn plant_at(terrain: &Terrain, x: i32, z: i32) -> Option<Block> {
    if terrain.world_type() != WorldType::Standard {
        return None;
    }
    let roll = (hash(terrain.seed(), x, z, SALT_PLANT) >> 8) as f32 / (1 << 24) as f32;
    if roll >= GRASS_DENSITY * MAX_GRASS_SCALE + FLOWER_DENSITY * MAX_FLOWER_SCALE {
        return None;
    }
    let biome = terrain.biome_at(x, z);
    let flowers = FLOWER_DENSITY * flower_scale(biome);
    if roll < flowers {
        return Some(flower_at(terrain, biome, x, z));
    }
    (roll - flowers < GRASS_DENSITY * grass_scale(biome)).then(|| grass_at(biome, x, z, terrain))
}

fn grass_at(biome: Biome, x: i32, z: i32, terrain: &Terrain) -> Block {
    let pick = hash(terrain.seed(), x, z, SALT_FERN);
    let fern = if biome.is_taiga() {
        pick % FERN_OUT_OF_TAIGA < FERN_IN_TAIGA
    } else if biome.is_jungle() {
        pick.is_multiple_of(FERN_ONE_IN_JUNGLE)
    } else {
        false
    };
    if fern { Block::Fern } else { Block::TallGrass }
}

fn flower_at(terrain: &Terrain, biome: Biome, x: i32, z: i32) -> Block {
    let pick = hash(terrain.seed(), x, z, SALT_KIND);
    match biome {
        Biome::FlowerForest => {
            let noise = terrain.color_noise(x, z, FLOWER_FOREST_NOISE_SCALE);
            let at = ((1.0 + noise) / 2.0).clamp(0.0, 0.9999);
            FLOWER_FOREST_PICKS[(at * FLOWER_FOREST_PICKS.len() as f32) as usize]
        }
        Biome::Swampland | Biome::SwamplandM => Block::BlueOrchid,
        Biome::Plains | Biome::SunflowerPlains => {
            if terrain.color_noise(x, z, TULIP_NOISE_SCALE) < TULIP_BELOW {
                PLAINS_TULIPS[(pick >> 4) as usize % PLAINS_TULIPS.len()]
            } else if !pick.is_multiple_of(3) {
                PLAINS_DAISIES[(pick >> 4) as usize % PLAINS_DAISIES.len()]
            } else {
                Block::Dandelion
            }
        }
        _ => {
            if !pick.is_multiple_of(3) {
                Block::Dandelion
            } else {
                Block::Poppy
            }
        }
    }
}

fn grass_scale(biome: Biome) -> f32 {
    use Biome::*;
    match biome {
        Jungle | JungleHills | JungleM | JungleEdge | JungleEdgeM => MAX_GRASS_SCALE,
        Plains | SunflowerPlains | Swampland | SwamplandM | Savanna | SavannaM | SavannaPlateau
        | SavannaPlateauM => 1.0,
        Forest | ForestHills | FlowerForest | BirchForest | BirchForestHills | BirchForestM
        | BirchForestHillsM | RoofedForest | RoofedForestM => 0.5,
        Taiga | TaigaHills | TaigaM | ColdTaiga | ColdTaigaHills | ColdTaigaM | MegaTaiga
        | MegaTaigaHills | MegaSpruceTaiga | MegaSpruceTaigaHills => 0.4,
        ExtremeHills | ExtremeHillsPlus | ExtremeHillsM | ExtremeHillsPlusM => 0.3,
        _ => 0.0,
    }
}

fn flower_scale(biome: Biome) -> f32 {
    use Biome::*;
    match biome {
        Plains | SunflowerPlains | Forest | ForestHills => 1.0,
        FlowerForest => FLOWER_FOREST_SCALE,
        Swampland | SwamplandM => SWAMP_FLOWER_SCALE,
        _ => 0.0,
    }
}

pub fn plant(terrain: &Terrain, key: ColumnPos, column: &mut Column) {
    let origin = key.origin();
    for lz in 0..SECTION_SIZE {
        for lx in 0..SECTION_SIZE {
            let Some(top) = (0..WORLD_HEIGHT)
                .rev()
                .find(|&y| column.get([lx, y, lz].into()) != Block::Air)
            else {
                continue;
            };
            let ground = glam::IVec3::new(lx, top, lz);
            let above = ground + glam::IVec3::Y;
            if !matches!(
                column.get(ground),
                Block::Grass | Block::Podzol | Block::CoarseDirt
            ) || top + 1 >= WORLD_HEIGHT
                || top <= WATER_LEVEL
                || column.get(above) != Block::Air
            {
                continue;
            }
            if let Some(block) = plant_at(terrain, origin.x + lx, origin.z + lz) {
                column.set(above, block);
            }
        }
    }
}

#[cfg(test)]
mod tests;
