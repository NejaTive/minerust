use std::f32::consts::{PI, TAU};

use glam::{IVec3, Vec2, Vec3};
use world::block::FluidKind;
use world::mob::{MobKind, MobRecord};

use crate::physics::{Collision, Input, Mode, Player};
use crate::rng::Rng;

pub struct MobSpec {
    pub width: f32,
    pub height: f32,
    pub eye: f32,
    pub walk_input: f32,
    pub shadow_radius: f32,
    pub fall_speed: Option<f32>,
}

pub const PIG: MobSpec = MobSpec {
    width: 0.9,
    height: 0.9,
    eye: 0.6,
    walk_input: 0.5,
    shadow_radius: 0.7,
    fall_speed: None,
};

pub const CHICKEN: MobSpec = MobSpec {
    width: 0.4,
    height: 0.7,
    eye: 0.7,
    walk_input: 0.5,
    shadow_radius: 0.3,
    fall_speed: Some(3.9),
};

pub const COW: MobSpec = MobSpec {
    width: 0.9,
    height: 1.4,
    eye: 1.3,
    walk_input: 0.4,
    shadow_radius: 0.7,
    fall_speed: None,
};

pub const SHEEP: MobSpec = MobSpec {
    width: 0.9,
    height: 1.3,
    eye: 1.235,
    walk_input: 0.46,
    shadow_radius: 0.7,
    fall_speed: None,
};

pub const MOOSHROOM: MobSpec = COW;

pub const POLAR_BEAR: MobSpec = MobSpec {
    width: 1.3,
    height: 1.4,
    eye: 1.19,
    walk_input: 0.5,
    shadow_radius: 1.0,
    fall_speed: None,
};

pub const WOLF: MobSpec = MobSpec {
    width: 0.6,
    height: 0.85,
    eye: 0.68,
    walk_input: 0.6,
    shadow_radius: 0.5,
    fall_speed: None,
};

pub const HORSE: MobSpec = MobSpec {
    width: 1.3964844,
    height: 1.6,
    eye: 1.52,
    walk_input: 0.5,
    shadow_radius: 1.0,
    fall_speed: None,
};

pub const DONKEY: MobSpec = HORSE;

pub const LLAMA: MobSpec = MobSpec {
    width: 0.9,
    height: 1.87,
    eye: 1.77,
    walk_input: 0.5,
    shadow_radius: 0.7,
    fall_speed: None,
};

pub fn spec(kind: MobKind) -> &'static MobSpec {
    match kind {
        MobKind::Pig => &PIG,
        MobKind::Chicken => &CHICKEN,
        MobKind::Cow => &COW,
        MobKind::Sheep => &SHEEP,
        MobKind::Mooshroom => &MOOSHROOM,
        MobKind::PolarBear => &POLAR_BEAR,
        MobKind::Wolf => &WOLF,
        MobKind::Horse => &HORSE,
        MobKind::Donkey => &DONKEY,
        MobKind::Llama => &LLAMA,
    }
}

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
const FLAP_GROUND_RATE: f32 = -1.0;
const FLAP_AIR_RATE: f32 = 4.0;
const FLAP_RATE_SCALE: f32 = 0.3;
const FLAP_KICK: f32 = 1.0;
const FLAP_DECAY: f32 = 0.9;
const FLAP_PHASE_SPEED: f32 = 2.0;

#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub feet: Vec3,
    pub yaw: f32,
    pub head_yaw: f32,
    pub head_pitch: f32,
    pub limb_phase: f32,
    pub limb_swing: f32,
    pub flap_phase: f32,
    pub flap_amount: f32,
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
            flap_phase: self.flap_phase + (next.flap_phase - self.flap_phase) * alpha,
            flap_amount: self.flap_amount + (next.flap_amount - self.flap_amount) * alpha,
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
    pub variant: u8,
    pub body: Player,
    pub yaw: f32,
    head_yaw: f32,
    head_pitch: f32,
    limb_phase: f32,
    limb_swing: f32,
    flap_phase: f32,
    flap_amount: f32,
    flap_speed: f32,
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
        let spec = spec(kind);
        let mut body = Player::sized(feet, Mode::Walking, spec.width, spec.height);
        body.auto_step = false;
        if let Some(fall_speed) = spec.fall_speed {
            body.terminal_speed = fall_speed;
        }
        let mut mob = Mob {
            kind,
            variant: 0,
            body,
            yaw,
            head_yaw: 0.0,
            head_pitch: 0.0,
            limb_phase: 0.0,
            limb_swing: 0.0,
            flap_phase: 0.0,
            flap_amount: 0.0,
            flap_speed: 0.0,
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
                flap_phase: 0.0,
                flap_amount: 0.0,
            },
        };
        mob.prev = mob.pose();
        mob
    }

    pub fn from_record(record: &MobRecord, seed: u32) -> Mob {
        Mob {
            variant: record.variant,
            ..Mob::new(record.kind, record.pos, record.yaw, seed)
        }
    }

    pub fn record(&self) -> MobRecord {
        MobRecord {
            kind: self.kind,
            pos: self.body.feet,
            yaw: self.yaw,
            variant: self.variant,
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
            flap_phase: self.flap_phase,
            flap_amount: self.flap_amount,
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
                    spec(self.kind).walk_input
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
        self.animate_wings();
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
        let spec = spec(self.kind);
        let reach = spec.width / 2.0 + HOP_PROBE;
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
        free(ahead, top, top + spec.height)
            && free(
                here,
                feet.y + spec.height,
                feet.y + HOP_HEIGHT + spec.height,
            )
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

    fn animate_wings(&mut self) {
        if spec(self.kind).fall_speed.is_none() {
            return;
        }
        let rate = if self.body.on_ground {
            FLAP_GROUND_RATE
        } else {
            FLAP_AIR_RATE
        };
        self.flap_amount =
            (self.flap_amount + rate * FLAP_RATE_SCALE / TICKS_PER_MC_TICK).clamp(0.0, 1.0);
        if !self.body.on_ground && self.flap_speed < FLAP_KICK {
            self.flap_speed = FLAP_KICK;
        }
        self.flap_speed *= FLAP_DECAY.powf(1.0 / TICKS_PER_MC_TICK);
        self.flap_phase += self.flap_speed * FLAP_PHASE_SPEED / TICKS_PER_MC_TICK;
    }

    fn animate_head<C, F>(&mut self, around: &Surroundings<C, F>)
    where
        C: Fn(IVec3) -> Collision,
        F: Fn(IVec3) -> Option<(FluidKind, f32)>,
    {
        let eye = self.body.feet + Vec3::Y * spec(self.kind).eye;
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
