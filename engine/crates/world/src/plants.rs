use crate::block::Block;
use crate::column::{Column, WORLD_HEIGHT};
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;
use crate::terrain::{Biome, Terrain, WATER_LEVEL, WorldType};
use crate::trees::hash;

const GRASS_DENSITY: f32 = 0.12;
const FLOWER_DENSITY: f32 = 0.01;
const MAX_GRASS_SCALE: f32 = 2.0;
const MAX_FLOWER_SCALE: f32 = 4.0;

const SALT_PLANT: u64 = 11;
const SALT_KIND: u64 = 12;

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
        let poppy = hash(terrain.seed(), x, z, SALT_KIND) & 1 == 0;
        return Some(if poppy {
            Block::Poppy
        } else {
            Block::Dandelion
        });
    }
    (roll - flowers < GRASS_DENSITY * grass_scale(biome)).then_some(Block::TallGrass)
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
        FlowerForest => MAX_FLOWER_SCALE,
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
            if column.get(ground) != Block::Grass
                || top + 1 >= WORLD_HEIGHT
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
