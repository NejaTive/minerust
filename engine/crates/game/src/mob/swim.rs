use std::f32::consts::{FRAC_PI_2, PI, TAU};

use glam::{IVec3, Vec3};
use world::block::FluidKind;

use super::{Mob, Surroundings, TICKS_PER_MC_TICK, angle_diff};
use crate::physics::{Collision, Input, TICK};
use crate::rng::Rng;

const DRIFT_SPEED: f32 = 0.2;
const DRIFT_VERTICAL: (f32, f32) = (-0.1, 0.1);
const BLOCKS_PER_MC_TICK: f32 = 20.0;
const REDIRECT_ONE_IN: u32 = 50;
const CYCLE_RESTART_ONE_IN: u32 = 10;
const CYCLE_SPEED: f32 = 0.2;
const BURST_FROM: f32 = 0.75;
const TENTACLE_SWING: f32 = PI * 0.25;
const SETTLE_SPIN_LATE: f32 = 0.99;
const SETTLE_SPEED: f32 = 0.9;
const SETTLE_SPIN: f32 = 0.8;
const SPIN_STEP: f32 = 1.5 * PI * PI / 180.0;
const TURN_BLEND: f32 = 0.1;
const PITCH_BLEND: f32 = 0.1;
const LAND_PITCH_BLEND: f32 = 0.02;
const SHORE_PROBE: f32 = 0.7;
const SURFACE_MARGIN: f32 = 0.1;

pub(super) struct Swim {
    rotation: f32,
    cycle_speed: f32,
    speed: f32,
    spin_speed: f32,
    pub(super) spin: f32,
    drift: Vec3,
    pub(super) pitch: f32,
    pub(super) tentacle: f32,
}

impl Swim {
    pub(super) fn new(seed: u32) -> Swim {
        Swim {
            rotation: 0.0,
            cycle_speed: new_cycle_speed(&mut Rng::new(seed ^ 0x5851_F42D)),
            speed: 0.0,
            spin_speed: 0.0,
            spin: 0.0,
            drift: Vec3::ZERO,
            pitch: 0.0,
            tentacle: 0.0,
        }
    }
}

fn new_cycle_speed(rng: &mut Rng) -> f32 {
    1.0 / (rng.unit() + 1.0) * CYCLE_SPEED
}

fn per_tick(rate: f32) -> f32 {
    rate.powf(1.0 / TICKS_PER_MC_TICK)
}

fn blend(rate: f32) -> f32 {
    1.0 - per_tick(1.0 - rate)
}

impl Mob {
    pub(super) fn in_water<C, F>(&self, around: &Surroundings<C, F>) -> bool
    where
        C: Fn(IVec3) -> Collision,
        F: Fn(IVec3) -> Option<(FluidKind, f32)>,
    {
        self.body
            .fluid_depth(around.fluid)
            .is_some_and(|(kind, depth)| kind == FluidKind::Water && depth > 0.0)
    }

    pub(super) fn tick_squid<C, F>(&mut self, around: &Surroundings<C, F>)
    where
        C: Fn(IVec3) -> Collision,
        F: Fn(IVec3) -> Option<(FluidKind, f32)>,
    {
        let in_water = self.in_water(around);
        self.advance_swim(in_water);
        if in_water {
            let velocity = self.swim_velocity(around);
            self.body.glide(velocity, around.collision);
        } else {
            let idle = Input {
                forward: 0.0,
                strafe: 0.0,
                jump: false,
                descend: false,
            };
            self.body
                .step_with(&idle, self.yaw, around.collision, around.fluid);
        }
    }

    fn advance_swim(&mut self, in_water: bool) {
        let swim = &mut self.swim;
        swim.rotation += swim.cycle_speed / TICKS_PER_MC_TICK;
        if swim.rotation > TAU {
            swim.rotation -= TAU;
            if self.rng.chance(CYCLE_RESTART_ONE_IN) {
                swim.cycle_speed = new_cycle_speed(&mut self.rng);
            }
        }
        if !in_water {
            swim.tentacle = 0.0;
            swim.pitch += (FRAC_PI_2 - swim.pitch) * blend(LAND_PITCH_BLEND);
            return;
        }
        if swim.rotation < PI {
            let phase = swim.rotation / PI;
            swim.tentacle = (phase * phase * PI).sin() * TENTACLE_SWING;
            if phase > BURST_FROM {
                swim.speed = 1.0;
                swim.spin_speed = 1.0;
            } else {
                swim.spin_speed *= per_tick(SETTLE_SPIN);
            }
        } else {
            swim.tentacle = 0.0;
            swim.speed *= per_tick(SETTLE_SPEED);
            swim.spin_speed *= per_tick(SETTLE_SPIN_LATE);
        }
        if swim.drift == Vec3::ZERO || self.rng.chance(REDIRECT_ONE_IN * TICKS_PER_MC_TICK as u32) {
            let angle = self.rng.range(0.0, TAU);
            swim.drift = Vec3::new(
                angle.cos() * DRIFT_SPEED,
                self.rng.range(DRIFT_VERTICAL.0, DRIFT_VERTICAL.1),
                angle.sin() * DRIFT_SPEED,
            );
        }
        swim.spin += SPIN_STEP * swim.spin_speed / TICKS_PER_MC_TICK;
        let motion = swim.drift * swim.speed;
        let horizontal = motion.x.hypot(motion.z);
        if horizontal > 1e-4 {
            self.yaw += angle_diff(motion.z.atan2(motion.x), self.yaw) * blend(TURN_BLEND);
        }
        swim.pitch += (horizontal.atan2(motion.y) - swim.pitch) * blend(PITCH_BLEND);
    }

    fn swim_velocity<C, F>(&mut self, around: &Surroundings<C, F>) -> Vec3
    where
        C: Fn(IVec3) -> Collision,
        F: Fn(IVec3) -> Option<(FluidKind, f32)>,
    {
        let wet = |point: Vec3| {
            (around.fluid)(point.floor().as_ivec3())
                .is_some_and(|(kind, _)| kind == FluidKind::Water)
        };
        let mut velocity = self.swim.drift * self.swim.speed * BLOCKS_PER_MC_TICK;
        let feet = self.body.feet;
        let height = self.body.height;
        let head = feet + Vec3::Y * (height + SURFACE_MARGIN + velocity.y.max(0.0) * TICK);
        if velocity.y > 0.0 && !wet(head) {
            velocity.y = 0.0;
        }
        let flat = Vec3::new(velocity.x, 0.0, velocity.z).normalize_or_zero();
        let probe = feet + Vec3::Y * height * 0.5 + flat * SHORE_PROBE;
        if flat != Vec3::ZERO && !wet(probe) {
            velocity.x = 0.0;
            velocity.z = 0.0;
            self.swim.drift = Vec3::ZERO;
        }
        velocity
    }
}
