use glam::{IVec3, Vec3};
use world::block::Block;
use world::column::WORLD_HEIGHT;
use world::mob::{MobKind, MobRecord};
use world::pos::ColumnPos;
use world::terrain::WorldType;

use super::Game;
use crate::daytime::sky_state;
use crate::mob::Mob;
use crate::rng::Rng;
use crate::spawning::{
    ATTEMPTS_PER_GROUP, CYCLE_GROUP_SIZE, CYCLE_REACH, CYCLE_TICKS, GENERATION_ONE_IN,
    GENERATION_REACH, GROUPS_PER_COLUMN, MIN_LIGHT, MIN_PLAYER_DISTANCE, RADIUS, animals, cap,
    group_size, pick, reach_offset,
};

const COLUMN_SIZE: i32 = 16;
const SEED_MIX: (u32, u32) = (0x85EB_CA6B, 0xC2B2_AE35);
const WARM_UP: usize = 4;
const START_HEIGHT_MARGIN: u32 = 15;

impl Game {
    pub(super) fn mob_fits(&self, mob: &Mob) -> bool {
        let (min, max) = mob.body.aabb();
        let (lo, hi) = (
            (min + 1e-3).floor().as_ivec3(),
            (max - 1e-3).floor().as_ivec3(),
        );
        (lo.x..=hi.x).all(|x| {
            (lo.y..=hi.y).all(|y| {
                (lo.z..=hi.z)
                    .all(|z| self.world.block(IVec3::new(x, y, z)).collision_height() == 0.0)
            })
        })
    }

    pub(super) fn populate_column(&mut self, key: ColumnPos) -> Vec<MobRecord> {
        let mut records = Vec::new();
        if self.terrain.world_type() != WorldType::Debug {
            records = self.generation_herds(key);
        }
        self.world.populate(key, records.clone());
        records
    }

    pub(super) fn advance_spawning(&mut self, ticks: u32) {
        self.spawn_ticks += ticks;
        while self.spawn_ticks >= CYCLE_TICKS {
            self.spawn_ticks -= CYCLE_TICKS;
            self.spawn_cycle();
        }
    }

    fn generation_herds(&self, key: ColumnPos) -> Vec<MobRecord> {
        let mut rng = column_rng(self.seed, key);
        let origin = key.origin();
        let center = COLUMN_SIZE / 2;
        let list = animals(self.terrain.biome_at(origin.x + center, origin.z + center));
        let mut records = Vec::new();
        while !list.is_empty() && rng.chance(GENERATION_ONE_IN) {
            let Some(entry) = pick(&mut rng, list) else {
                break;
            };
            let kind = entry.kind;
            let group = group_size(&mut rng, entry);
            let mut local = (
                rng.below(COLUMN_SIZE as u32) as i32,
                rng.below(COLUMN_SIZE as u32) as i32,
            );
            for _ in 0..group {
                for _ in 0..ATTEMPTS_PER_GROUP {
                    let (x, z) = (origin.x + local.0, origin.z + local.1);
                    let placed = self.grass_surface(x, z).and_then(|feet| {
                        let mob = self.new_mob(kind, feet, &mut rng);
                        self.mob_fits(&mob).then(|| mob.record())
                    });
                    local = shifted_within_column(&mut rng, local);
                    if let Some(record) = placed {
                        records.push(record);
                        break;
                    }
                }
            }
        }
        records
    }

    fn grass_surface(&self, x: i32, z: i32) -> Option<IVec3> {
        let top = (0..WORLD_HEIGHT).rev().find(|&y| {
            let block = self.world.block(IVec3::new(x, y, z));
            block.collision_height() > 0.0 || block.fluid().is_some()
        })?;
        spawn_ground(self.world.block(IVec3::new(x, top, z))).then(|| IVec3::new(x, top + 1, z))
    }

    fn new_mob(&self, kind: MobKind, cell: IVec3, rng: &mut Rng) -> Mob {
        let feet = Vec3::new(cell.x as f32 + 0.5, cell.y as f32, cell.z as f32 + 0.5);
        let yaw = rng.range(0.0, std::f32::consts::TAU);
        Mob::new(kind, feet, yaw, rng.next_u32())
    }

    fn spawn_cycle(&mut self) {
        if self.terrain.world_type() == WorldType::Debug {
            return;
        }
        let center = self.camera_column();
        let mut keys: Vec<ColumnPos> = self
            .world
            .keys()
            .filter(|key| {
                (key.x() - center.x()).abs() <= RADIUS && (key.z() - center.z()).abs() <= RADIUS
            })
            .collect();
        keys.sort_by_key(|key| (key.x(), key.z()));
        let limit = cap(keys.len());
        let mut rng = Rng::new(self.mob_rng.next_u32());
        for key in keys {
            if self.mobs.len() >= limit {
                break;
            }
            self.spawn_in_column(key, limit, &mut rng);
        }
    }

    fn spawn_in_column(&mut self, key: ColumnPos, limit: usize, rng: &mut Rng) {
        let origin = key.origin();
        let x = origin.x + rng.below(COLUMN_SIZE as u32) as i32;
        let z = origin.z + rng.below(COLUMN_SIZE as u32) as i32;
        let height = self.terrain.surface_height(x, z).max(0) as u32;
        let y = rng.below(height + START_HEIGHT_MARGIN) as i32;
        if self.world.block(IVec3::new(x, y, z)).collision_height() > 0.0 {
            return;
        }
        let mut spawned = 0;
        for _ in 0..GROUPS_PER_COLUMN {
            let (mut gx, mut gz) = (x, z);
            let mut chosen: Option<MobKind> = None;
            for _ in 0..ATTEMPTS_PER_GROUP {
                gx += reach_offset(rng, CYCLE_REACH);
                gz += reach_offset(rng, CYCLE_REACH);
                let cell = IVec3::new(gx, y, gz);
                if !self.world.is_loaded(cell) {
                    continue;
                }
                if chosen.is_none() {
                    chosen = pick(rng, animals(self.terrain.biome_at(gx, gz))).map(|e| e.kind);
                }
                let Some(kind) = chosen else {
                    break;
                };
                if self.mobs.len() >= limit {
                    return;
                }
                let mob = self.new_mob(kind, cell, rng);
                if self.may_spawn_here(&mob, cell) {
                    self.mobs.push(mob);
                    spawned += 1;
                    if spawned >= CYCLE_GROUP_SIZE {
                        return;
                    }
                }
            }
        }
    }

    fn may_spawn_here(&self, mob: &Mob, cell: IVec3) -> bool {
        spawn_ground(self.world.block(cell - IVec3::Y))
            && self.spawn_light(cell) >= MIN_LIGHT
            && mob.body.feet.distance(self.player.feet) >= MIN_PLAYER_DISTANCE
            && self.mob_fits(mob)
    }

    fn spawn_light(&self, cell: IVec3) -> u8 {
        let darkness = sky_state(self.day.fraction()).darkness.round() as u8;
        self.light
            .block(cell)
            .max(self.light.sky(cell).saturating_sub(darkness))
    }
}

fn spawn_ground(block: Block) -> bool {
    block.is_grass() || block == Block::Mycelium
}

fn column_rng(seed: i32, key: ColumnPos) -> Rng {
    let mixed = (seed as u32)
        ^ (key.x() as u32).wrapping_mul(SEED_MIX.0)
        ^ (key.z() as u32).wrapping_mul(SEED_MIX.1);
    let mut rng = Rng::new(mixed);
    for _ in 0..WARM_UP {
        rng.next_u32();
    }
    rng
}

fn shifted_within_column(rng: &mut Rng, from: (i32, i32)) -> (i32, i32) {
    loop {
        let next = (
            from.0 + reach_offset(rng, GENERATION_REACH),
            from.1 + reach_offset(rng, GENERATION_REACH),
        );
        if (0..COLUMN_SIZE).contains(&next.0) && (0..COLUMN_SIZE).contains(&next.1) {
            return next;
        }
    }
}

#[cfg(test)]
mod tests;
