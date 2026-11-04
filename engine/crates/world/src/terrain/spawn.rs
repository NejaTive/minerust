use glam::IVec3;

use super::{Biome, Terrain, WATER_LEVEL};
use crate::column::WORLD_HEIGHT;
use crate::pos::ColumnPos;
use crate::section::SECTION_SIZE;

const SEARCH_RADIUS: i32 = 256;
const SEARCH_ATTEMPTS: usize = 1000;
const SHIFT_RANGE: u64 = 64;
const BIOME_CELL_SHIFT: i32 = 2;
const SPAWN_SALT: u64 = 0x5EED_0F5B_A3D1_C0DE;
const HEADROOM: i32 = 2;

pub(super) const SPAWN_BIOMES: [Biome; 7] = [
    Biome::Forest,
    Biome::Plains,
    Biome::Taiga,
    Biome::TaigaHills,
    Biome::ForestHills,
    Biome::Jungle,
    Biome::JungleHills,
];

struct Splitmix(u64);

impl Splitmix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        (self.next() >> 11) % bound
    }
}

impl Terrain {
    /// `BiomeProvider.findBiomePosition` + `WorldServer.createSpawnPosition`: блок земли, на который встаёт игрок.
    pub fn spawn_point(&self) -> IVec3 {
        let mut rng = Splitmix(u64::from(self.seed as u32) ^ SPAWN_SALT);
        let (mut x, mut z) = self.find_spawn_biome(&mut rng).unwrap_or((0, 0));
        let mut ground = self.spawn_ground(x, z);
        for _ in 0..SEARCH_ATTEMPTS {
            if let Some(y) = ground {
                return IVec3::new(x, y, z);
            }
            x += rng.below(SHIFT_RANGE) as i32 - rng.below(SHIFT_RANGE) as i32;
            z += rng.below(SHIFT_RANGE) as i32 - rng.below(SHIFT_RANGE) as i32;
            ground = self.spawn_ground(x, z);
        }
        IVec3::new(x, self.surface_height(x, z), z)
    }

    fn find_spawn_biome(&self, rng: &mut Splitmix) -> Option<(i32, i32)> {
        let first = -SEARCH_RADIUS >> BIOME_CELL_SHIFT;
        let side = ((SEARCH_RADIUS >> BIOME_CELL_SHIFT) - first + 1) as usize;
        let coarse = self.map.coarse(first, first, side, side);
        let mut found = None;
        let mut count = 0;
        for cz in first..first + side as i32 {
            for cx in first..first + side as i32 {
                let allowed = Biome::from_id(coarse.at(cx, cz))
                    .is_some_and(|biome| SPAWN_BIOMES.contains(&biome));
                if !allowed {
                    continue;
                }
                if rng.below(count + 1) == 0 {
                    found = Some((cx << BIOME_CELL_SHIFT, cz << BIOME_CELL_SHIFT));
                }
                count += 1;
            }
        }
        found
    }

    fn spawn_ground(&self, x: i32, z: i32) -> Option<i32> {
        if !SPAWN_BIOMES.contains(&self.biome_at(x, z)) || self.surface_height(x, z) <= WATER_LEVEL
        {
            return None;
        }
        let column = self.generate(ColumnPos::of_block(IVec3::new(x, 0, z)));
        let local = IVec3::new(x.rem_euclid(SECTION_SIZE), 0, z.rem_euclid(SECTION_SIZE));
        let at = |y| column.get(IVec3::new(local.x, y, local.z));
        let ground = (0..WORLD_HEIGHT).rev().find(|&y| {
            let block = at(y);
            block.is_solid() && !block.is_leaves() && !block.is_log()
        })?;
        let clear =
            (1..=HEADROOM).all(|d| ground + d >= WORLD_HEIGHT || !at(ground + d).is_solid());
        (at(ground).is_grass() && clear).then_some(ground)
    }
}
