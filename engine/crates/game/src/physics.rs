use glam::{IVec3, Vec2, Vec3};

use world::block::WATER_SURFACE;

pub const PLAYER_WIDTH: f32 = 0.6;
pub const PLAYER_HEIGHT: f32 = 1.8;
pub const EYE_HEIGHT: f32 = 1.62;
pub const TICK: f32 = 1.0 / 60.0;

pub const GRAVITY: f32 = 32.0;
pub const TERMINAL_SPEED: f32 = 78.0;
pub const JUMP_SPEED: f32 = 9.0;
pub const WALK_SPEED: f32 = 4.3;
pub const FLY_SPEED: f32 = 8.0;
pub const SPRINT_FACTOR: f32 = 1.3;
pub const FLY_SPRINT_FACTOR: f32 = 2.0;
pub const SPRINT_FOV_SCALE: f32 = 1.1;
pub const SPRINT_FOV_TIME: f32 = 0.15;
pub const AIR_ACCEL: f32 = 10.0;

pub const WATER_GRAVITY: f32 = GRAVITY / 4.0;
pub const WATER_SINK_SPEED: f32 = 2.0;
pub const SWIM_UP_SPEED: f32 = 3.0;
const SWIM_UP_ACCEL: f32 = 24.0;
pub const SWIM_SPEED: f32 = WALK_SPEED * 0.5;
const WATER_ACCEL: f32 = 20.0;
const WATER_DRAG: f32 = 0.928;
pub const SWIM_DEPTH: f32 = 0.4;
pub const WATER_EXIT_SPEED: f32 = 10.0;
const WATER_EXIT_DEPTH: f32 = 1.0;

const EPS: f32 = 1e-3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Walking,
    Flying,
    Noclip,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Input {
    pub forward: f32,
    pub strafe: f32,
    pub jump: bool,
    pub descend: bool,
}

pub struct Player {
    pub feet: Vec3,
    pub velocity: Vec3,
    pub mode: Mode,
    pub on_ground: bool,
    pub sprinting: bool,
}

impl Player {
    pub fn new(feet: Vec3, mode: Mode) -> Self {
        Self {
            feet,
            velocity: Vec3::ZERO,
            mode,
            on_ground: false,
            sprinting: false,
        }
    }

    pub fn start_sprint(&mut self) {
        self.sprinting = true;
    }

    pub fn aabb(&self) -> (Vec3, Vec3) {
        let half = PLAYER_WIDTH / 2.0;
        (
            self.feet - Vec3::new(half, 0.0, half),
            self.feet + Vec3::new(half, PLAYER_HEIGHT, half),
        )
    }

    pub fn eye(&self) -> Vec3 {
        self.feet + Vec3::Y * EYE_HEIGHT
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            Mode::Walking => {
                self.velocity.y = 0.0;
                Mode::Flying
            }
            Mode::Flying | Mode::Noclip => Mode::Walking,
        };
    }

    pub fn toggle_noclip(&mut self) {
        self.mode = match self.mode {
            Mode::Noclip => Mode::Flying,
            Mode::Walking | Mode::Flying => {
                self.velocity.y = 0.0;
                Mode::Noclip
            }
        };
    }

    pub fn overlaps_block(&self, pos: IVec3) -> bool {
        let (min, max) = self.aabb();
        let block_min = pos.as_vec3();
        let block_max = block_min + Vec3::ONE;
        (min + EPS).cmplt(block_max).all() && (max - EPS).cmpgt(block_min).all()
    }

    pub fn water_depth(&self, is_water: impl Fn(IVec3) -> bool) -> f32 {
        let (min, max) = self.aabb();
        let lo = (min + EPS).floor().as_ivec3();
        let hi = (max - EPS).floor().as_ivec3();
        let mut surface = None;
        for y in lo.y..=hi.y {
            for z in lo.z..=hi.z {
                for x in lo.x..=hi.x {
                    let pos = IVec3::new(x, y, z);
                    if is_water(pos) {
                        let top = y as f32 + water_height(pos, &is_water);
                        surface = Some(surface.map_or(top, |s: f32| s.max(top)));
                    }
                }
            }
        }
        surface.map_or(0.0, |s| (s - self.feet.y).max(0.0))
    }

    pub fn eyes_in_water(&self, is_water: impl Fn(IVec3) -> bool) -> bool {
        let eye = self.eye();
        let cell = eye.floor().as_ivec3();
        is_water(cell) && eye.y < cell.y as f32 + water_height(cell, &is_water)
    }

    pub fn step(
        &mut self,
        input: &Input,
        yaw: f32,
        is_solid: impl Fn(IVec3) -> bool,
        is_water: impl Fn(IVec3) -> bool,
    ) {
        let (sin, cos) = yaw.sin_cos();
        let forward = Vec3::new(cos, 0.0, sin);
        let right = Vec3::new(-sin, 0.0, cos);
        let wish = (forward * input.forward + right * input.strafe).clamp_length_max(1.0);
        let vertical = f32::from(u8::from(input.jump)) - f32::from(u8::from(input.descend));

        let depth = match self.mode {
            Mode::Walking => self.water_depth(&is_water),
            Mode::Flying | Mode::Noclip => 0.0,
        };
        if input.forward <= 0.0 || depth > 0.0 {
            self.sprinting = false;
        }
        match self.mode {
            Mode::Walking if depth > 0.0 => {
                let target = Vec2::new(wish.x, wish.z) * SWIM_SPEED;
                let horizontal = Vec2::new(self.velocity.x, self.velocity.z)
                    .move_towards(target, WATER_ACCEL * TICK);
                self.velocity.x = horizontal.x;
                self.velocity.z = horizontal.y;
                let mut vy = self.velocity.y - WATER_GRAVITY * TICK;
                if input.jump && depth > SWIM_DEPTH && vy < SWIM_UP_SPEED {
                    vy = (vy + SWIM_UP_ACCEL * TICK).min(SWIM_UP_SPEED);
                }
                if vy < -WATER_SINK_SPEED {
                    vy = (vy * WATER_DRAG).min(-WATER_SINK_SPEED);
                }
                self.velocity.y = vy;
            }
            Mode::Walking => {
                let speed = if self.sprinting {
                    WALK_SPEED * SPRINT_FACTOR
                } else {
                    WALK_SPEED
                };
                let target = Vec2::new(wish.x, wish.z) * speed;
                let horizontal = if self.on_ground {
                    target
                } else {
                    Vec2::new(self.velocity.x, self.velocity.z)
                        .move_towards(target, AIR_ACCEL * TICK)
                };
                self.velocity.x = horizontal.x;
                self.velocity.z = horizontal.y;
                if self.on_ground && input.jump {
                    self.velocity.y = JUMP_SPEED;
                }
                self.velocity.y = (self.velocity.y - GRAVITY * TICK).max(-TERMINAL_SPEED);
            }
            Mode::Flying | Mode::Noclip => {
                let speed = if self.sprinting {
                    FLY_SPEED * FLY_SPRINT_FACTOR
                } else {
                    FLY_SPEED
                };
                self.velocity = (wish + Vec3::Y * vertical).clamp_length_max(1.0) * speed;
            }
        }

        let delta = self.velocity * TICK;
        self.on_ground = false;
        if self.mode == Mode::Noclip {
            self.feet += delta;
            return;
        }
        let mut against_wall = false;
        for axis in [1, 0, 2] {
            let blocked = self.move_axis(axis, delta[axis], &is_solid);
            against_wall |= blocked && axis != 1;
        }
        if against_wall {
            self.sprinting = false;
        }
        if depth > 0.0 && depth < WATER_EXIT_DEPTH && input.jump && against_wall {
            self.velocity.y = self.velocity.y.max(WATER_EXIT_SPEED);
        }
        if self.mode == Mode::Flying && self.on_ground {
            self.mode = Mode::Walking;
        }
    }

    fn move_axis(&mut self, axis: usize, delta: f32, is_solid: &impl Fn(IVec3) -> bool) -> bool {
        if delta == 0.0 {
            return false;
        }
        let (min, max) = self.aabb();
        let (below, above) = if axis == 1 {
            (0.0, PLAYER_HEIGHT)
        } else {
            (PLAYER_WIDTH / 2.0, PLAYER_WIDTH / 2.0)
        };
        let lo = (min + EPS).floor().as_ivec3();
        let hi = (max - EPS).floor().as_ivec3();
        let blocked = |layer: i32| {
            let mut pos = lo;
            pos[axis] = layer;
            let (a, b) = match axis {
                0 => (1, 2),
                1 => (0, 2),
                _ => (0, 1),
            };
            (lo[a]..=hi[a]).any(|i| {
                (lo[b]..=hi[b]).any(|j| {
                    pos[a] = i;
                    pos[b] = j;
                    is_solid(pos)
                })
            })
        };

        if delta > 0.0 {
            let first = (max[axis] - EPS).ceil() as i32;
            let last = (max[axis] + delta - EPS).floor() as i32;
            if let Some(layer) = (first..=last).find(|&layer| blocked(layer)) {
                self.feet[axis] = layer as f32 - above;
                self.velocity[axis] = 0.0;
                return true;
            }
        } else {
            let first = (min[axis] + EPS).floor() as i32 - 1;
            let last = (min[axis] + delta + EPS).floor() as i32;
            if let Some(layer) = (last..=first).rev().find(|&layer| blocked(layer)) {
                self.feet[axis] = (layer + 1) as f32 + below;
                self.velocity[axis] = 0.0;
                if axis == 1 {
                    self.on_ground = true;
                }
                return true;
            }
        }
        self.feet[axis] += delta;
        false
    }
}

fn water_height(pos: IVec3, is_water: &impl Fn(IVec3) -> bool) -> f32 {
    if is_water(pos + IVec3::Y) {
        1.0
    } else {
        WATER_SURFACE
    }
}

pub fn next_fov_scale(scale: f32, sprinting: bool, dt: f32) -> f32 {
    let target = if sprinting { SPRINT_FOV_SCALE } else { 1.0 };
    let step = (SPRINT_FOV_SCALE - 1.0) * dt / SPRINT_FOV_TIME;
    if scale < target {
        (scale + step).min(target)
    } else {
        (scale - step).max(target)
    }
}

#[derive(Default)]
pub struct Clock {
    acc: f32,
}

impl Clock {
    pub fn advance(&mut self, dt: f32) -> u32 {
        self.acc += dt;
        let ticks = (self.acc / TICK).floor() as u32;
        self.acc = (self.acc - ticks as f32 * TICK).max(0.0);
        ticks
    }

    pub fn alpha(&self) -> f32 {
        (self.acc / TICK).min(1.0)
    }

    pub fn reset(&mut self) {
        self.acc = 0.0;
    }
}

#[cfg(test)]
mod tests;
