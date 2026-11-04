use world::mob::{HORSE_COLOURS, HORSE_MARKINGS, LLAMA_COLOURS, MobKind, horse_variant};
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
const HORSE_PLAINS: SpawnEntry = entry(MobKind::Horse, 5, 2, 6);
const DONKEY_PLAINS: SpawnEntry = entry(MobKind::Donkey, 1, 1, 3);
const HORSE_SAVANNA: SpawnEntry = entry(MobKind::Horse, 1, 2, 6);
const DONKEY_SAVANNA: SpawnEntry = entry(MobKind::Donkey, 1, 1, 1);
const LLAMA_PLATEAU: SpawnEntry = entry(MobKind::Llama, 8, 4, 4);
const LLAMA_HILLS: SpawnEntry = entry(MobKind::Llama, 5, 4, 6);

const PLAINS_ANIMALS: [SpawnEntry; 6] = [SHEEP, PIG, CHICKEN, COW, HORSE_PLAINS, DONKEY_PLAINS];
const SAVANNA_ANIMALS: [SpawnEntry; 6] = [SHEEP, PIG, CHICKEN, COW, HORSE_SAVANNA, DONKEY_SAVANNA];
const PLATEAU_ANIMALS: [SpawnEntry; 7] = [
    SHEEP,
    PIG,
    CHICKEN,
    COW,
    HORSE_SAVANNA,
    DONKEY_SAVANNA,
    LLAMA_PLATEAU,
];
const HILLS_ANIMALS: [SpawnEntry; 5] = [SHEEP, PIG, CHICKEN, COW, LLAMA_HILLS];
const MUSHROOM_ANIMALS: [SpawnEntry; 1] = [entry(MobKind::Mooshroom, 8, 4, 8)];
const SNOW_ANIMALS: [SpawnEntry; 1] = [entry(MobKind::PolarBear, 1, 1, 2)];

pub fn animals(biome: Biome) -> &'static [SpawnEntry] {
    match biome {
        Biome::MushroomIsland | Biome::MushroomIslandShore => &MUSHROOM_ANIMALS,
        Biome::IcePlains | Biome::IceMountains | Biome::IceSpikes => &SNOW_ANIMALS,
        Biome::Plains | Biome::SunflowerPlains => &PLAINS_ANIMALS,
        Biome::Savanna | Biome::SavannaM | Biome::SavannaPlateauM => &SAVANNA_ANIMALS,
        Biome::SavannaPlateau => &PLATEAU_ANIMALS,
        Biome::ExtremeHills
        | Biome::ExtremeHillsEdge
        | Biome::ExtremeHillsPlus
        | Biome::ExtremeHillsM
        | Biome::ExtremeHillsPlusM => &HILLS_ANIMALS,
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

pub fn group_colour(rng: &mut Rng, kind: MobKind) -> u8 {
    match kind {
        MobKind::Horse => rng.below(u32::from(HORSE_COLOURS)) as u8,
        MobKind::Llama => rng.below(u32::from(LLAMA_COLOURS)) as u8,
        _ => 0,
    }
}

pub fn variant_in_group(rng: &mut Rng, kind: MobKind, colour: u8) -> u8 {
    match kind {
        MobKind::Horse => horse_variant(colour, rng.below(u32::from(HORSE_MARKINGS)) as u8),
        _ => colour,
    }
}

pub fn random_variant(rng: &mut Rng, kind: MobKind) -> u8 {
    let colour = group_colour(rng, kind);
    variant_in_group(rng, kind, colour)
}

pub fn reach_offset(rng: &mut Rng, reach: u32) -> i32 {
    rng.below(reach) as i32 - rng.below(reach) as i32
}

pub fn cap(columns: usize) -> usize {
    CREATURE_CAP * columns / CAP_COLUMNS
}

#[cfg(test)]
mod tests;
