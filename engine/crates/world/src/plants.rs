use glam::IVec3;

use crate::block::Block;
use crate::column::{Column, WORLD_HEIGHT};
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;
use crate::stream::Stream;
use crate::terrain::{Biome, Terrain, WATER_LEVEL, WorldType};
use crate::trees::hash;

const SALT_PLANTS: u64 = 11;
const SKY_REACH: u32 = 32;
const FLOWER_TRIES: u32 = 64;
const GRASS_TRIES: u32 = 128;
const SPREAD: u32 = 8;
const VERTICAL_SPREAD: u32 = 4;
const CENTER: i32 = 8;
const TULIP_NOISE_SCALE: f32 = 200.0;
const TULIP_BELOW: f32 = -0.55;
const FLOWER_FOREST_NOISE_SCALE: f32 = 48.0;
const FERN_IN_TAIGA: u32 = 4;
const FERN_OF_TAIGA: u32 = 5;
const FERN_ONE_IN_JUNGLE: u32 = 4;
const DANDELION_ONE_IN_THREE: u32 = 3;
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

pub(crate) struct Attempts {
    pub flowers: u32,
    pub grass: u32,
}

/// `flowersPerChunk` и `grassPerChunk` из `BiomeDecorator` и биомов 1.12.
pub(crate) fn attempts(biome: Biome) -> Attempts {
    use Biome::*;
    let (flowers, grass) = match biome {
        _ if biome.is_mushroom_island() || biome.is_desert() => (0, 0),
        _ if biome.is_mesa() => (0, 1),
        FlowerForest => (100, 2),
        _ if biome.is_forest() => (2, 2),
        Plains | SunflowerPlains => (4, 10),
        Savanna | SavannaM | SavannaPlateau | SavannaPlateauM => (4, 20),
        _ if biome.is_swamp() => (1, 5),
        _ if biome.is_jungle() => (4, 25),
        _ if biome.is_mega_taiga() => (2, 7),
        _ => (2, 1),
    };
    Attempts { flowers, grass }
}

fn pick_flower(terrain: &Terrain, biome: Biome, rng: &mut Stream, x: i32, z: i32) -> Block {
    match biome {
        Biome::FlowerForest => {
            let noise = terrain.color_noise(x, z, FLOWER_FOREST_NOISE_SCALE);
            let at = ((1.0 + noise) / 2.0).clamp(0.0, 0.9999);
            FLOWER_FOREST_PICKS[(at * FLOWER_FOREST_PICKS.len() as f32) as usize]
        }
        _ if biome.is_swamp() => Block::BlueOrchid,
        Biome::Plains | Biome::SunflowerPlains => {
            if terrain.color_noise(x, z, TULIP_NOISE_SCALE) < TULIP_BELOW {
                PLAINS_TULIPS[rng.below(PLAINS_TULIPS.len() as u32) as usize]
            } else if rng.below(DANDELION_ONE_IN_THREE) != 0 {
                PLAINS_DAISIES[rng.below(PLAINS_DAISIES.len() as u32) as usize]
            } else {
                Block::Dandelion
            }
        }
        _ => {
            if rng.below(DANDELION_ONE_IN_THREE) != 0 {
                Block::Dandelion
            } else {
                Block::Poppy
            }
        }
    }
}

fn pick_grass(biome: Biome, rng: &mut Stream) -> Block {
    let fern = if biome.is_taiga() {
        rng.below(FERN_OF_TAIGA) < FERN_IN_TAIGA
    } else if biome.is_jungle() {
        rng.below(FERN_ONE_IN_JUNGLE) == 0
    } else {
        false
    };
    if fern { Block::Fern } else { Block::TallGrass }
}

fn highest(column: &Column, x: i32, z: i32) -> Option<i32> {
    (0..WORLD_HEIGHT)
        .rev()
        .find(|&y| column.get([x, y, z].into()) != Block::Air)
}

fn start(rng: &mut Stream, column: &Column) -> Option<IVec3> {
    let x = rng.below(SECTION_SIZE as u32) as i32;
    let z = rng.below(SECTION_SIZE as u32) as i32;
    let top = highest(column, x, z)?;
    let y = rng.below((top + 1) as u32 + SKY_REACH) as i32;
    Some(IVec3::new(x, y, z))
}

fn grow_cluster(rng: &mut Stream, column: &mut Column, center: IVec3, block: Block, tries: u32) {
    for _ in 0..tries {
        let at = center
            + IVec3::new(
                rng.spread(SPREAD),
                rng.spread(VERTICAL_SPREAD),
                rng.spread(SPREAD),
            );
        if at.y - 1 > WATER_LEVEL
            && Column::contains(at)
            && column.get(at) == Block::Air
            && matches!(
                column.get(at - IVec3::Y),
                Block::Grass | Block::Podzol | Block::CoarseDirt
            )
        {
            column.set(at, block);
        }
    }
}

pub fn plant(terrain: &Terrain, key: ColumnPos, column: &mut Column) {
    if terrain.world_type() != WorldType::Standard {
        return;
    }
    let origin = key.origin();
    let biome = terrain.biome_at(origin.x + CENTER, origin.z + CENTER);
    let counts = attempts(biome);
    let state = u64::from(hash(terrain.seed(), key.x(), key.z(), SALT_PLANTS))
        | u64::from(hash(terrain.seed(), key.x(), key.z(), SALT_PLANTS + 100)) << 32;
    let mut rng = Stream::new(state);
    for _ in 0..counts.flowers {
        let Some(center) = start(&mut rng, column) else {
            continue;
        };
        let flower = pick_flower(
            terrain,
            biome,
            &mut rng,
            origin.x + center.x,
            origin.z + center.z,
        );
        grow_cluster(&mut rng, column, center, flower, FLOWER_TRIES);
    }
    for _ in 0..counts.grass {
        let Some(mut center) = start(&mut rng, column) else {
            continue;
        };
        let block = pick_grass(biome, &mut rng);
        while center.y > 0 && matches!(column.get(center), b if b == Block::Air || b.is_leaves()) {
            center.y -= 1;
        }
        grow_cluster(&mut rng, column, center, block, GRASS_TRIES);
    }
}

#[cfg(test)]
mod tests;
