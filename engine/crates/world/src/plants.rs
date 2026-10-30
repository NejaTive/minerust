use crate::block::Block;
use crate::column::{Column, WORLD_HEIGHT};
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;
use crate::terrain::{Biome, Terrain, WATER_LEVEL};
use crate::trees::hash;

const GRASS_DENSITY: f32 = 0.12;
const FLOWER_DENSITY: f32 = 0.01;

const SALT_PLANT: u64 = 11;
const SALT_KIND: u64 = 12;

fn plant_at(terrain: &Terrain, x: i32, z: i32) -> Option<Block> {
    let roll = (hash(terrain.seed(), x, z, SALT_PLANT) >> 8) as f32 / (1 << 24) as f32;
    if roll >= GRASS_DENSITY + FLOWER_DENSITY {
        return None;
    }
    if !matches!(terrain.biome_at(x, z), Biome::Plains | Biome::Forest) {
        return None;
    }
    if roll < FLOWER_DENSITY {
        let poppy = hash(terrain.seed(), x, z, SALT_KIND) & 1 == 0;
        return Some(if poppy {
            Block::Poppy
        } else {
            Block::Dandelion
        });
    }
    Some(Block::TallGrass)
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
