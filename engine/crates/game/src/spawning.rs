use world::mob::MobKind;
use world::terrain::Biome;

use crate::physics::TICK;
use crate::rng::Rng;

pub const CYCLE_TICKS: u32 = (400.0 / 20.0 / TICK + 0.5) as u32;
pub const CREATURE_CAP: usize = 10;
pub const CAP_COLUMNS: usize = 289;
pub const RADIUS: i32 = 8;
pub const MIN_PLAYER_DISTANCE: f32 = 24.0;
pub const MIN_LIGHT: u8 = 9;
pub const GENERATION_ONE_IN: u32 = 10;
pub const CYCLE_GROUP_SIZE: u32 = 4;
pub const GROUPS_PER_COLUMN: u32 = 3;
pub const ATTEMPTS_PER_GROUP: u32 = 4;
pub const GENERATION_REACH: u32 = 5;
pub const CYCLE_REACH: u32 = 6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SpawnEntry {
    pub kind: MobKind,
    pub weight: u32,
    pub min: u32,
    pub max: u32,
}

const fn entry(kind: MobKind, weight: u32, min: u32, max: u32) -> SpawnEntry {
    SpawnEntry {
        kind,
        weight,
        min,
        max,
    }
}

const SHEEP: SpawnEntry = entry(MobKind::Sheep, 12, 4, 4);
const PIG: SpawnEntry = entry(MobKind::Pig, 10, 4, 4);
const CHICKEN: SpawnEntry = entry(MobKind::Chicken, 10, 4, 4);
const COW: SpawnEntry = entry(MobKind::Cow, 8, 4, 4);

const FARM_ANIMALS: [SpawnEntry; 4] = [SHEEP, PIG, CHICKEN, COW];
const FOREST_ANIMALS: [SpawnEntry; 5] = [SHEEP, PIG, CHICKEN, COW, entry(MobKind::Wolf, 5, 4, 4)];
const TAIGA_ANIMALS: [SpawnEntry; 5] = [SHEEP, PIG, CHICKEN, COW, entry(MobKind::Wolf, 8, 4, 4)];
const MUSHROOM_ANIMALS: [SpawnEntry; 1] = [entry(MobKind::Mooshroom, 8, 4, 8)];
const SNOW_ANIMALS: [SpawnEntry; 1] = [entry(MobKind::PolarBear, 1, 1, 2)];

pub fn animals(biome: Biome) -> &'static [SpawnEntry] {
    match biome {
        Biome::MushroomIsland | Biome::MushroomIslandShore => &MUSHROOM_ANIMALS,
        Biome::IcePlains | Biome::IceMountains | Biome::IceSpikes => &SNOW_ANIMALS,
        Biome::Forest | Biome::ForestHills => &FOREST_ANIMALS,
        Biome::Taiga
        | Biome::TaigaHills
        | Biome::TaigaM
        | Biome::ColdTaiga
        | Biome::ColdTaigaHills
        | Biome::ColdTaigaM
        | Biome::MegaTaiga
        | Biome::MegaTaigaHills
        | Biome::MegaSpruceTaiga
        | Biome::MegaSpruceTaigaHills => &TAIGA_ANIMALS,
        _ if biome.is_ocean()
            || biome.is_river()
            || biome.is_desert()
            || biome.is_mesa()
            || matches!(biome, Biome::Beach | Biome::StoneBeach | Biome::ColdBeach) =>
        {
            &[]
        }
        _ => &FARM_ANIMALS,
    }
}

pub fn pick(rng: &mut Rng, list: &[SpawnEntry]) -> Option<SpawnEntry> {
    let total: u32 = list.iter().map(|entry| entry.weight).sum();
    if total == 0 {
        return None;
    }
    let mut roll = rng.below(total);
    list.iter().find_map(|&entry| {
        if roll < entry.weight {
            Some(entry)
        } else {
            roll -= entry.weight;
            None
        }
    })
}

pub fn group_size(rng: &mut Rng, entry: SpawnEntry) -> u32 {
    entry.min + rng.below(1 + entry.max - entry.min)
}

pub fn reach_offset(rng: &mut Rng, reach: u32) -> i32 {
    rng.below(reach) as i32 - rng.below(reach) as i32
}

pub fn cap(columns: usize) -> usize {
    CREATURE_CAP * columns / CAP_COLUMNS
}

#[cfg(test)]
mod tests;
