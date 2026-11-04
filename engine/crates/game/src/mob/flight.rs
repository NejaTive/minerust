use std::f32::consts::TAU;

use glam::{IVec3, Vec2, Vec3};
use world::block::FluidKind;

use super::{Mob, Plan, Surroundings, TURN_RATE, turn_towards};
use crate::physics::Collision;

const FLY_ONE_IN: u32 = 480;
const FLY_SPEED: f32 = 4.0;
const FLIGHT_RANGE: (f32, f32) = (5.0, 14.0);
const FLIGHT_TRIES: u32 = 4;
const SCAN_UP: i32 = 8;
const SCAN_DOWN: i32 = 12;
const CRUISE_HEIGHT: (f32, f32) = (1.5, 4.0);
const LAND_ONE_IN: u32 = 2;
const ARRIVE_DISTANCE: f32 = 0.6;
const FLIGHT_MAX_TICKS: u32 = 1200;
const STUCK_CHECK_TICKS: u32 = 60;
const STUCK_DISTANCE: f32 = 0.5;
const OBSTACLE_PROBE: f32 = 0.6;
const CLIMB_SLOWDOWN: f32 = 0.3;
const CLIMB_SPEED: f32 = 3.5;

impl Mob {
    /// Возвращает `true`, если в этом тике попугай летел и обычное движение выполнять не нужно.
    pub(super) fn tick_flight<C, F>(&mut self, around: &Surroundings<C, F>) -> bool
    where
        C: Fn(IVec3) -> Collision,
        F: Fn(IVec3) -> Option<(FluidKind, f32)>,
    {
        if !matches!(self.plan, Plan::Fly { .. }) {
            if !self.rng.chance(FLY_ONE_IN) {
                return false;
            }
            let Some(target) = self.pick_flight(around) else {
                return false;
            };
            self.plan = Plan::Fly {
                target,
                ticks: 0,
                checkpoint: self.body.feet,
            };
        }
        let Plan::Fly {
            target,
            ticks,
            checkpoint,
        } = self.plan
        else {
            return false;
        };
        let ticks = ticks + 1;
        let feet = self.body.feet;
        let to_target = target - feet;
        let mut checkpoint = checkpoint;
        let mut stuck = false;
        if ticks % STUCK_CHECK_TICKS == 0 {
            stuck = feet.distance(checkpoint) < STUCK_DISTANCE;
            checkpoint = feet;
        }
        if to_target.length() < ARRIVE_DISTANCE || ticks > FLIGHT_MAX_TICKS || stuck {
            self.plan = Plan::Idle;
            return false;
        }
        self.plan = Plan::Fly {
            target,
            ticks,
            checkpoint,
        };
        let heading = to_target.normalize();
        self.yaw = turn_towards(self.yaw, heading.z.atan2(heading.x), TURN_RATE);
        let mut velocity = heading * FLY_SPEED;
        if self.blocked_ahead(around, heading) {
            velocity.x *= CLIMB_SLOWDOWN;
            velocity.z *= CLIMB_SLOWDOWN;
            velocity.y = velocity.y.max(CLIMB_SPEED);
        }
        self.body.glide(velocity, around.collision);
        true
    }

    fn blocked_ahead<C, F>(&self, around: &Surroundings<C, F>, heading: Vec3) -> bool
    where
        C: Fn(IVec3) -> Collision,
        F: Fn(IVec3) -> Option<(FluidKind, f32)>,
    {
        let flat = Vec3::new(heading.x, 0.0, heading.z).normalize_or_zero();
        let reach = self.body.width / 2.0 + OBSTACLE_PROBE;
        let probe = self.body.feet + Vec3::Y * self.body.height / 2.0 + flat * reach;
        (around.collision)(probe.floor().as_ivec3()).height > 0.0
    }

    pub(super) fn pick_flight<C, F>(&mut self, around: &Surroundings<C, F>) -> Option<Vec3>
    where
        C: Fn(IVec3) -> Collision,
        F: Fn(IVec3) -> Option<(FluidKind, f32)>,
    {
        let here = Vec2::new(self.body.feet.x, self.body.feet.z);
        let base = self.body.feet.y.floor() as i32;
        let mut best: Option<Vec3> = None;
        for _ in 0..FLIGHT_TRIES {
            let angle = self.rng.range(0.0, TAU);
            let distance = self.rng.range(FLIGHT_RANGE.0, FLIGHT_RANGE.1);
            let spot = here + Vec2::from_angle(angle) * distance;
            let column = IVec3::new(spot.x.floor() as i32, 0, spot.y.floor() as i32);
            let Some(top) = self.landing_top(around, column, base) else {
                continue;
            };
            let point = Vec3::new(spot.x, top, spot.y);
            if best.is_none_or(|prev| point.y > prev.y) {
                best = Some(point);
            }
        }
        let mut target = best?;
        if !self.rng.chance(LAND_ONE_IN) {
            target.y += self.rng.range(CRUISE_HEIGHT.0, CRUISE_HEIGHT.1);
        }
        Some(target)
    }

    fn landing_top<C, F>(
        &self,
        around: &Surroundings<C, F>,
        column: IVec3,
        base: i32,
    ) -> Option<f32>
    where
        C: Fn(IVec3) -> Collision,
        F: Fn(IVec3) -> Option<(FluidKind, f32)>,
    {
        let at = |y: i32| IVec3::new(column.x, y, column.z);
        if (around.collision)(at(base + SCAN_UP)).height > 0.0 {
            return None;
        }
        let y = (base - SCAN_DOWN..base + SCAN_UP)
            .rev()
            .find(|&y| (around.collision)(at(y)).height > 0.0)?;
        let wet = |y: i32| (around.fluid)(at(y)).is_some();
        (!wet(y) && !wet(y + 1)).then(|| y as f32 + (around.collision)(at(y)).height)
    }
}
