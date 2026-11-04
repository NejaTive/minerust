use world::mob::{
    HORSE_COLOURS, HORSE_MARKINGS, LLAMA_COLOURS, MobKind, PARROT_COLOURS, RABBIT_BLACK,
    RABBIT_BROWN, RABBIT_GOLD, RABBIT_SALT, RABBIT_SPLOTCHED, RABBIT_WHITE, horse_variant,
};
use world::terrain::{Biome, WATER_LEVEL};

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
pub const OCELOT_MIN_Y: i32 = WATER_LEVEL + 1;
pub const WATER_CAP: usize = 5;
pub const WATER_MIN_Y: i32 = 45;
pub const WATER_COLUMNS_PER_TICK: u32 = 6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    Creature,
    Water,
}

pub fn category(kind: MobKind) -> Category {
    match kind {
        MobKind::Squid => Category::Water,
        _ => Category::Creature,
    }
}

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
const RABBIT: SpawnEntry = entry(MobKind::Rabbit, 4, 2, 3);
const RABBIT_SNOW: SpawnEntry = entry(MobKind::Rabbit, 10, 2, 3);
const POLAR_BEAR: SpawnEntry = entry(MobKind::PolarBear, 1, 1, 2);

const FOREST_ANIMALS: [SpawnEntry; 5] = [SHEEP, PIG, CHICKEN, COW, entry(MobKind::Wolf, 5, 4, 4)];
const FLOWER_FOREST_ANIMALS: [SpawnEntry; 5] = [SHEEP, PIG, CHICKEN, COW, RABBIT];
const TAIGA_ANIMALS: [SpawnEntry; 6] = [
    SHEEP,
    PIG,
    CHICKEN,
    COW,
    entry(MobKind::Wolf, 8, 4, 4),
    RABBIT,
];
const JUNGLE_ANIMALS: [SpawnEntry; 7] = [
    SHEEP,
    PIG,
    CHICKEN,
    COW,
    entry(MobKind::Chicken, 10, 4, 4),
    entry(MobKind::Ocelot, 2, 1, 1),
    entry(MobKind::Parrot, 40, 1, 2),
];
const WATER_CREATURES: [SpawnEntry; 1] = [entry(MobKind::Squid, 10, 4, 4)];
const DESERT_ANIMALS: [SpawnEntry; 1] = [RABBIT];
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
const SNOW_ANIMALS: [SpawnEntry; 2] = [RABBIT_SNOW, POLAR_BEAR];

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
        Biome::FlowerForest => &FLOWER_FOREST_ANIMALS,
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
        _ if biome.is_desert() => &DESERT_ANIMALS,
        _ if biome.is_jungle() => &JUNGLE_ANIMALS,
        _ if biome.is_ocean()
            || biome.is_river()
            || biome.is_mesa()
            || matches!(biome, Biome::Beach | Biome::StoneBeach | Biome::ColdBeach) =>
        {
            &[]
        }
        _ => &FARM_ANIMALS,
    }
}

pub fn water_creatures(_biome: Biome) -> &'static [SpawnEntry] {
    &WATER_CREATURES
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

pub fn group_colour(rng: &mut Rng, kind: MobKind, biome: Biome) -> u8 {
    match kind {
        MobKind::Horse => rng.below(u32::from(HORSE_COLOURS)) as u8,
        MobKind::Llama => rng.below(u32::from(LLAMA_COLOURS)) as u8,
        MobKind::Rabbit => rabbit_colour(rng, biome),
        _ => 0,
    }
}

fn rabbit_colour(rng: &mut Rng, biome: Biome) -> u8 {
    let roll = rng.below(100);
    if biome.is_snowy() {
        if roll < 80 {
            RABBIT_WHITE
        } else {
            RABBIT_SPLOTCHED
        }
    } else if biome.is_desert() {
        RABBIT_GOLD
    } else if roll < 50 {
        RABBIT_BROWN
    } else if roll < 90 {
        RABBIT_SALT
    } else {
        RABBIT_BLACK
    }
}

pub fn variant_in_group(rng: &mut Rng, kind: MobKind, colour: u8) -> u8 {
    match kind {
        MobKind::Horse => horse_variant(colour, rng.below(u32::from(HORSE_MARKINGS)) as u8),
        MobKind::Parrot => rng.below(u32::from(PARROT_COLOURS)) as u8,
        _ => colour,
    }
}

pub fn random_variant(rng: &mut Rng, kind: MobKind, biome: Biome) -> u8 {
    let colour = group_colour(rng, kind, biome);
    variant_in_group(rng, kind, colour)
}

pub fn reach_offset(rng: &mut Rng, reach: u32) -> i32 {
    rng.below(reach) as i32 - rng.below(reach) as i32
}

pub fn cap(columns: usize) -> usize {
    CREATURE_CAP * columns / CAP_COLUMNS
}

pub fn water_cap(columns: usize) -> usize {
    WATER_CAP * columns / CAP_COLUMNS
}

#[cfg(test)]
mod tests;
