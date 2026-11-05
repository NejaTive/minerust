use glam::{IVec3, Vec3};
use world::block::Block;
use world::explosion::{TNT_POWER, destroy, destroyed_cells, knockback};
use world::plates::{Body, Plates};
use world::signal::Signal;
use world::tripwires::Tripwires;

use super::{Game, TntView};
use crate::physics::EYE_HEIGHT;
use crate::rng::Rng;
use crate::tnt::{self, FLASH_TICKS, FUSE_TICKS, MC_TICK, PrimedTnt};

const MC_TICKS_PER_SECOND: f32 = 20.0;
const CHAIN_FUSE_BASE: u32 = FUSE_TICKS as u32 / 8;
const CHAIN_FUSE_SPREAD: u32 = FUSE_TICKS as u32 / 4;
const MOB_EYE_SHARE: f32 = 0.85;

pub(super) struct Blasts {
    pub(super) signal: Signal,
    pub(super) plates: Plates,
    pub(super) tripwires: Tripwires,
    pub(super) tnts: Vec<PrimedTnt>,
    acc: u32,
    rng: Rng,
}

impl Blasts {
    pub(super) fn new(seed: i32) -> Self {
        Self {
            signal: Signal::new(),
            plates: Plates::new(),
            tripwires: Tripwires::new(),
            tnts: Vec::new(),
            acc: 0,
            rng: Rng::new(seed as u32 ^ 0x7F4A_7C15),
        }
    }
}

impl Game {
    pub(super) fn ignite(&mut self, pos: IVec3) -> Vec<IVec3> {
        let changed = self.world.set_block_supported(pos, Block::Air);
        if !changed.is_empty() {
            self.prime(pos, FUSE_TICKS);
        }
        changed
    }

    fn prime(&mut self, pos: IVec3, fuse: i32) {
        let tnt = PrimedTnt::at(pos, fuse, &mut self.blasts.rng);
        self.blasts.tnts.push(tnt);
    }

    pub fn tnt_views(&self) -> Vec<TntView> {
        let alpha = (self.blasts.acc as f32 + self.clock.alpha()) / MC_TICK as f32;
        self.blasts
            .tnts
            .iter()
            .map(|tnt| {
                let feet = tnt.prev.lerp(tnt.pos, alpha.min(1.0));
                let cell = (feet + Vec3::Y * 0.5).floor().as_ivec3();
                let left = tnt.fuse as f32 - alpha + 1.0;
                TntView {
                    feet,
                    scale: tnt::swell(left),
                    flash: tnt.fuse / FLASH_TICKS % 2 == 0,
                    sky_level: self.light.sky(cell),
                    block_level: self.light.block(cell),
                }
            })
            .collect()
    }

    pub fn primed_tnt_count(&self) -> usize {
        self.blasts.tnts.len()
    }

    pub(super) fn advance_blasts(&mut self, ticks: u32) {
        self.blasts.acc += ticks;
        while self.blasts.acc >= MC_TICK {
            self.blasts.acc -= MC_TICK;
            self.blast_tick();
        }
    }

    fn blast_tick(&mut self) {
        let mut bodies: Vec<Body> = vec![self.player.aabb()];
        bodies.extend(self.mobs.iter().map(|mob| mob.body.aabb()));
        let pressed = self.blasts.plates.tick(&mut self.world, &bodies);
        self.apply_pending(&pressed);
        let tripped = self.blasts.tripwires.tick(&mut self.world, &bodies);
        self.apply_pending(&tripped);

        let ignition = self.blasts.signal.step(&mut self.world);
        self.apply_pending(&ignition.changed);
        for pos in ignition.ignited {
            self.prime(pos, FUSE_TICKS);
        }

        let world = &self.world;
        let mut exploding = Vec::new();
        self.blasts.tnts.retain_mut(|tnt| {
            let done = tnt.tick(|pos| super::cell_collision(world, pos));
            if done {
                exploding.push(tnt.blast_center());
            }
            !done
        });
        for center in exploding {
            self.explode(center);
        }
    }

    fn explode(&mut self, center: Vec3) {
        let rng = &mut self.blasts.rng;
        let cells = destroyed_cells(&self.world, center, TNT_POWER, || rng.unit());
        self.push_from_blast(center);
        let destruction = destroy(&mut self.world, &cells);
        self.apply_pending(&destruction.changed);
        for pos in destruction.tnt {
            let fuse = CHAIN_FUSE_BASE + self.blasts.rng.below(CHAIN_FUSE_SPREAD);
            self.prime(pos, fuse as i32);
        }
    }

    fn push_from_blast(&mut self, center: Vec3) {
        let world = &self.world;
        let kick = |body, eye| knockback(world, center, TNT_POWER, body, eye) * MC_TICKS_PER_SECOND;
        let player = kick(self.player.aabb(), EYE_HEIGHT);
        let mobs: Vec<Vec3> = self
            .mobs
            .iter()
            .map(|mob| kick(mob.body.aabb(), mob.body.height * MOB_EYE_SHARE))
            .collect();
        let tnts: Vec<Vec3> = self
            .blasts
            .tnts
            .iter()
            .map(|tnt| {
                world::explosion::knockback(
                    world,
                    center,
                    TNT_POWER,
                    tnt.aabb(),
                    crate::tnt::BLAST_LIFT,
                )
            })
            .collect();
        if player != Vec3::ZERO {
            self.player.velocity += player;
            self.player.on_ground = false;
        }
        for (mob, push) in self.mobs.iter_mut().zip(mobs) {
            if push != Vec3::ZERO {
                mob.body.velocity += push;
                mob.body.on_ground = false;
            }
        }
        for (tnt, push) in self.blasts.tnts.iter_mut().zip(tnts) {
            tnt.velocity += push;
        }
    }
}
