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
pub const GROUP_SIZE: u32 = 4;
pub const GROUPS_PER_COLUMN: u32 = 3;
pub const ATTEMPTS_PER_GROUP: u32 = 4;
pub const GENERATION_REACH: u32 = 5;
pub const CYCLE_REACH: u32 = 6;

const FARM_ANIMALS: [(MobKind, u32); 4] = [
    (MobKind::Sheep, 12),
    (MobKind::Pig, 10),
    (MobKind::Chicken, 10),
    (MobKind::Cow, 8),
];

pub fn animals(biome: Biome) -> &'static [(MobKind, u32)] {
    let barren = biome.is_ocean()
        || biome.is_river()
        || biome.is_desert()
        || biome.is_mesa()
        || matches!(
            biome,
            Biome::Beach
                | Biome::StoneBeach
                | Biome::ColdBeach
                | Biome::MushroomIsland
                | Biome::MushroomIslandShore
        );
    if barren { &[] } else { &FARM_ANIMALS }
}

pub fn pick(rng: &mut Rng, list: &[(MobKind, u32)]) -> Option<MobKind> {
    let total: u32 = list.iter().map(|(_, weight)| weight).sum();
    if total == 0 {
        return None;
    }
    let mut roll = rng.below(total);
    list.iter().find_map(|&(kind, weight)| {
        if roll < weight {
            Some(kind)
        } else {
            roll -= weight;
            None
        }
    })
}

pub fn reach_offset(rng: &mut Rng, reach: u32) -> i32 {
    rng.below(reach) as i32 - rng.below(reach) as i32
}

pub fn cap(columns: usize) -> usize {
    CREATURE_CAP * columns / CAP_COLUMNS
}
