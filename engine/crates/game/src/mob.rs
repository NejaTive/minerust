use std::f32::consts::{PI, TAU};

use glam::{IVec3, Vec2, Vec3};
use world::block::FluidKind;
use world::mob::{MobKind, MobRecord};

use crate::physics::{Collision, Input, Mode, Player};
use crate::rng::Rng;

pub const PIG_SIZE: f32 = 0.9;

pub struct MobSpec {
    pub size: f32,
    pub shadow_radius: f32,
}

pub const PIG: MobSpec = MobSpec {
    size: PIG_SIZE,
    shadow_radius: 0.7,
};

pub fn spec(kind: MobKind) -> &'static MobSpec {
    match kind {
        MobKind::Pig => &PIG,
    }
}
const PIG_EYE: f32 = 0.6;

const WALK_INPUT: f32 = 0.5;
const TURN_RATE: f32 = 0.12;
const FACING_TOLERANCE: f32 = 1.0;
const ARRIVE_DISTANCE: f32 = 0.6;
const STROLL_MIN: f32 = 3.0;
const STROLL_MAX: f32 = 10.0;
const STROLL_ONE_IN: u32 = 240;
const STROLL_MAX_TICKS: u32 = 900;
const STUCK_CHECK_TICKS: u32 = 60;
const STUCK_DISTANCE: f32 = 0.3;
const SAFE_DROP: i32 = 3;
const CLIFF_PROBE: f32 = 0.7;
const HOP_PROBE: f32 = 0.15;
const HOP_HEIGHT: f32 = 1.0;
const HOP_EPS: f32 = 1e-3;
const WATCH_RANGE: f32 = 6.0;
const WATCH_ONE_IN: u32 = 90;
const WATCH_TICKS: (f32, f32) = (120.0, 240.0);
const LOOK_ONE_IN: u32 = 150;
const LOOK_TICKS: (f32, f32) = (40.0, 120.0);
const LOOK_YAW: f32 = 1.2;
const HEAD_YAW_LIMIT: f32 = 1.31;
const HEAD_PITCH_LIMIT: f32 = 0.7;
const HEAD_RATE: f32 = 0.08;
const LIMB_BLEND: f32 = 0.15;
const TICKS_PER_MC_TICK: f32 = 3.0;

#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub feet: Vec3,
    pub yaw: f32,
    pub head_yaw: f32,
    pub head_pitch: f32,
    pub limb_phase: f32,
    pub limb_swing: f32,
}

impl Pose {
    pub fn lerp(self, next: Pose, alpha: f32) -> Pose {
        Pose {
            feet: self.feet.lerp(next.feet, alpha),
            yaw: lerp_angle(self.yaw, next.yaw, alpha),
            head_yaw: self.head_yaw + (next.head_yaw - self.head_yaw) * alpha,
            head_pitch: self.head_pitch + (next.head_pitch - self.head_pitch) * alpha,
            limb_phase: self.limb_phase + (next.limb_phase - self.limb_phase) * alpha,
            limb_swing: self.limb_swing + (next.limb_swing - self.limb_swing) * alpha,
        }
    }
}

#[derive(Clone, Copy)]
enum Plan {
    Idle,
    Walk {
        target: Vec2,
        ticks: u32,
        checkpoint: Vec2,
    },
}

pub struct Mob {
    pub kind: MobKind,
    pub body: Player,
    pub yaw: f32,
    head_yaw: f32,
    head_pitch: f32,
    limb_phase: f32,
    limb_swing: f32,
    plan: Plan,
    head_goal: Vec2,
    head_hold: u32,
    watching: u32,
    rng: Rng,
    prev: Pose,
}

pub struct Surroundings<'a, C, F> {
    pub collision: &'a C,
    pub fluid: &'a F,
    pub player_eye: Vec3,
}

impl Mob {
    pub fn new(kind: MobKind, feet: Vec3, yaw: f32, seed: u32) -> Mob {
        let mut body = Player::sized(feet, Mode::Walking, spec(kind).size, spec(kind).size);
        body.auto_step = false;
        let mut mob = Mob {
            kind,
            body,
            yaw,
            head_yaw: 0.0,
            head_pitch: 0.0,
            limb_phase: 0.0,
            limb_swing: 0.0,
            plan: Plan::Idle,
            head_goal: Vec2::ZERO,
            head_hold: 0,
            watching: 0,
            rng: Rng::new(seed),
            prev: Pose {
                feet,
                yaw,
                head_yaw: 0.0,
                head_pitch: 0.0,
                limb_phase: 0.0,
                limb_swing: 0.0,
            },
        };
        mob.prev = mob.pose();
        mob
    }

    pub fn from_record(record: &MobRecord, seed: u32) -> Mob {
        Mob::new(record.kind, record.pos, record.yaw, seed)
    }

    pub fn record(&self) -> MobRecord {
        MobRecord {
            kind: self.kind,
            pos: self.body.feet,
            yaw: self.yaw,
        }
    }

    pub fn pose(&self) -> Pose {
        Pose {
            feet: self.body.feet,
            yaw: self.yaw,
            head_yaw: self.head_yaw,
            head_pitch: self.head_pitch,
            limb_phase: self.limb_phase,
            limb_swing: self.limb_swing,
        }
    }

    pub fn interpolated(&self, alpha: f32) -> Pose {
        self.prev.lerp(self.pose(), alpha)
    }

    pub fn tick<C, F>(&mut self, around: &Surroundings<C, F>)
    where
        C: Fn(IVec3) -> Collision,
        F: Fn(IVec3) -> Option<(FluidKind, f32)>,
    {
        self.prev = self.pose();
        let swimming = self
            .body
            .fluid_depth(around.fluid)
            .is_some_and(|(_, depth)| depth > 0.0);
        let heading = self.think(around);
        let forward = match heading {
            Some(goal) => {
                self.yaw = turn_towards(self.yaw, goal, TURN_RATE);
                let aligned = angle_diff(goal, self.yaw).abs() < FACING_TOLERANCE;
                if aligned && self.ground_ahead(around) {
                    WALK_INPUT
                } else {
                    0.0
                }
            }
            None => 0.0,
        };
        let before = self.body.feet;
        let input = Input {
            forward,
            strafe: 0.0,
            jump: swimming || forward > 0.0 && self.body.on_ground && self.can_hop(around),
            descend: false,
        };
        self.body
            .step_with(&input, self.yaw, around.collision, around.fluid);
        let moved = Vec2::new(self.body.feet.x - before.x, self.body.feet.z - before.z).length();
        self.animate_limbs(moved);
        self.animate_head(around);
    }

    fn think<C, F>(&mut self, around: &Surroundings<C, F>) -> Option<f32>
    where
        C: Fn(IVec3) -> Collision,
        F: Fn(IVec3) -> Option<(FluidKind, f32)>,
    {
        let here = Vec2::new(self.body.feet.x, self.body.feet.z);
        match self.plan {
            Plan::Idle => {
                if self.rng.chance(STROLL_ONE_IN) {
                    let angle = self.rng.range(0.0, TAU);
                    let distance = self.rng.range(STROLL_MIN, STROLL_MAX);
                    self.plan = Plan::Walk {
                        target: here + Vec2::from_angle(angle) * distance,
                        ticks: 0,
                        checkpoint: here,
                    };
                }
                None
            }
            Plan::Walk {
                target,
                ticks,
                checkpoint,
            } => {
                let ticks = ticks + 1;
                let to_target = target - here;
                let mut arrived = to_target.length() < ARRIVE_DISTANCE || ticks > STROLL_MAX_TICKS;
                let mut checkpoint = checkpoint;
                if ticks % STUCK_CHECK_TICKS == 0 {
                    arrived |= here.distance(checkpoint) < STUCK_DISTANCE;
                    checkpoint = here;
                }
                if arrived || !self.ground_ahead(around) && self.body.on_ground {
                    self.plan = Plan::Idle;
                    return None;
                }
                self.plan = Plan::Walk {
                    target,
                    ticks,
                    checkpoint,
                };
                Some(to_target.y.atan2(to_target.x))
            }
        }
    }

    fn can_hop<C, F>(&self, around: &Surroundings<C, F>) -> bool
    where
        C: Fn(IVec3) -> Collision,
        F: Fn(IVec3) -> Option<(FluidKind, f32)>,
    {
        let feet = self.body.feet;
        let reach = PIG_SIZE / 2.0 + HOP_PROBE;
        let ahead = (feet + Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin()) * reach)
            .floor()
            .as_ivec3();
        let here = feet.floor().as_ivec3();
        let base = (feet.y + HOP_EPS).floor() as i32;
        let wall = (around.collision)(IVec3::new(ahead.x, base, ahead.z)).height;
        let top = base as f32 + wall;
        if wall <= 0.0 || top - feet.y > HOP_HEIGHT + HOP_EPS {
            return false;
        }
        let free = |column: IVec3, from: f32, to: f32| {
            let (lo, hi) = (
                (from + HOP_EPS).floor() as i32,
                (to - HOP_EPS).floor() as i32,
            );
            (lo..=hi).all(|y| (around.collision)(IVec3::new(column.x, y, column.z)).height <= 0.0)
        };
        free(ahead, top, top + PIG_SIZE)
            && free(here, feet.y + PIG_SIZE, feet.y + HOP_HEIGHT + PIG_SIZE)
    }

    fn ground_ahead<C, F>(&self, around: &Surroundings<C, F>) -> bool
    where
        C: Fn(IVec3) -> Collision,
        F: Fn(IVec3) -> Option<(FluidKind, f32)>,
    {
        let probe = self.body.feet + Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin()) * CLIFF_PROBE;
        let top = probe.floor().as_ivec3();
        (0..=SAFE_DROP + 1).any(|depth| {
            let cell = top - IVec3::Y * depth;
            (around.collision)(cell).height > 0.0 || (around.fluid)(cell).is_some()
        })
    }

    fn animate_limbs(&mut self, moved: f32) {
        let target = (moved * TICKS_PER_MC_TICK * 4.0).min(1.0);
        self.limb_swing += (target - self.limb_swing) * LIMB_BLEND;
        self.limb_phase += self.limb_swing / TICKS_PER_MC_TICK;
    }

    fn animate_head<C, F>(&mut self, around: &Surroundings<C, F>)
    where
        C: Fn(IVec3) -> Collision,
        F: Fn(IVec3) -> Option<(FluidKind, f32)>,
    {
        let eye = self.body.feet + Vec3::Y * PIG_EYE;
        let to_player = around.player_eye - eye;
        let flat = Vec2::new(to_player.x, to_player.z);
        if self.watching > 0 {
            self.watching -= 1;
        } else if flat.length() < WATCH_RANGE && self.rng.chance(WATCH_ONE_IN) {
            self.watching = self.rng.range(WATCH_TICKS.0, WATCH_TICKS.1) as u32;
        }
        if self.watching > 0 && flat.length() < WATCH_RANGE {
            let yaw = angle_diff(flat.y.atan2(flat.x), self.yaw);
            let pitch = to_player.y.atan2(flat.length().max(0.01));
            self.head_goal = Vec2::new(yaw, pitch);
        } else if self.head_hold > 0 {
            self.head_hold -= 1;
        } else if self.rng.chance(LOOK_ONE_IN) {
            self.head_goal = Vec2::new(
                self.rng.range(-LOOK_YAW, LOOK_YAW),
                self.rng.range(-0.3, 0.3),
            );
            self.head_hold = self.rng.range(LOOK_TICKS.0, LOOK_TICKS.1) as u32;
        } else {
            self.head_goal = Vec2::ZERO;
        }
        let goal_yaw = self.head_goal.x.clamp(-HEAD_YAW_LIMIT, HEAD_YAW_LIMIT);
        let goal_pitch = self.head_goal.y.clamp(-HEAD_PITCH_LIMIT, HEAD_PITCH_LIMIT);
        self.head_yaw += (goal_yaw - self.head_yaw).clamp(-HEAD_RATE, HEAD_RATE);
        self.head_pitch += (goal_pitch - self.head_pitch).clamp(-HEAD_RATE, HEAD_RATE);
    }
}

pub fn angle_diff(to: f32, from: f32) -> f32 {
    (to - from + PI).rem_euclid(TAU) - PI
}

fn turn_towards(from: f32, to: f32, max_step: f32) -> f32 {
    from + angle_diff(to, from).clamp(-max_step, max_step)
}

fn lerp_angle(a: f32, b: f32, alpha: f32) -> f32 {
    a + angle_diff(b, a) * alpha
}

#[cfg(test)]
mod tests;
