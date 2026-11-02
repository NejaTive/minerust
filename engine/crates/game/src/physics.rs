use glam::{IVec3, Vec2, Vec3};
use world::block::FluidKind;

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
pub const ICE_ACCEL: f32 = 3.0;
pub const STEP_HEIGHT: f32 = 1.0;
pub const STEP_SMOOTH_TIME: f32 = 0.12;

pub const WATER_GRAVITY: f32 = GRAVITY / 4.0;
pub const SWIM_DEPTH: f32 = 0.4;
const WATER_EXIT_DEPTH: f32 = 1.0;

pub struct Medium {
    pub sink_speed: f32,
    pub swim_speed: f32,
    pub swim_up_speed: f32,
    pub swim_up_accel: f32,
    pub accel: f32,
    pub drag: f32,
    pub exit_speed: f32,
}

pub const WATER_SINK_SPEED: f32 = 2.0;
pub const SWIM_UP_SPEED: f32 = 3.0;
pub const SWIM_SPEED: f32 = WALK_SPEED * 0.5;
pub const WATER_EXIT_SPEED: f32 = 10.0;
pub const LAVA_SINK_SPEED: f32 = 1.0;
pub const LAVA_SWIM_SPEED: f32 = SWIM_SPEED * 0.5;
pub const LAVA_SWIM_UP_SPEED: f32 = SWIM_UP_SPEED * 0.5;
pub const LAVA_EXIT_SPEED: f32 = 6.0;

pub const WATER: Medium = Medium {
    sink_speed: WATER_SINK_SPEED,
    swim_speed: SWIM_SPEED,
    swim_up_speed: SWIM_UP_SPEED,
    swim_up_accel: 24.0,
    accel: 20.0,
    drag: 0.928,
    exit_speed: WATER_EXIT_SPEED,
};

pub const LAVA: Medium = Medium {
    sink_speed: LAVA_SINK_SPEED,
    swim_speed: LAVA_SWIM_SPEED,
    swim_up_speed: LAVA_SWIM_UP_SPEED,
    swim_up_accel: 12.0,
    accel: 10.0,
    drag: 0.86,
    exit_speed: LAVA_EXIT_SPEED,
};

pub fn medium(kind: FluidKind) -> &'static Medium {
    match kind {
        FluidKind::Water => &WATER,
        FluidKind::Lava => &LAVA,
    }
}

const EPS: f32 = 1e-3;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Collision {
    pub height: f32,
    pub slippery: bool,
}

impl Collision {
    pub const NONE: Collision = Collision {
        height: 0.0,
        slippery: false,
    };
    pub const FULL: Collision = Collision {
        height: 1.0,
        slippery: false,
    };
}

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
    pub auto_step: bool,
    pub step_offset: f32,
}

impl Player {
    pub fn new(feet: Vec3, mode: Mode) -> Self {
        Self {
            feet,
            velocity: Vec3::ZERO,
            mode,
            on_ground: false,
            sprinting: false,
            auto_step: true,
            step_offset: 0.0,
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
                self.step_offset = 0.0;
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
                self.step_offset = 0.0;
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

    pub fn fluid_depth(
        &self,
        fluid: impl Fn(IVec3) -> Option<(FluidKind, f32)>,
    ) -> Option<(FluidKind, f32)> {
        let (min, max) = self.aabb();
        let lo = (min + EPS).floor().as_ivec3();
        let hi = (max - EPS).floor().as_ivec3();
        let mut found: Option<(FluidKind, f32)> = None;
        for y in lo.y..=hi.y {
            for z in lo.z..=hi.z {
                for x in lo.x..=hi.x {
                    let pos = IVec3::new(x, y, z);
                    if let Some((kind, height)) = fluid(pos) {
                        let top = y as f32 + height;
                        found = Some(match found {
                            Some((prev, surface)) if prev == kind => (kind, surface.max(top)),
                            Some((FluidKind::Lava, surface)) => (FluidKind::Lava, surface),
                            _ => (kind, top),
                        });
                    }
                }
            }
        }
        found.map(|(kind, surface)| (kind, (surface - self.feet.y).max(0.0)))
    }

    pub fn eye_fluid(
        &self,
        fluid: impl Fn(IVec3) -> Option<(FluidKind, f32)>,
    ) -> Option<FluidKind> {
        let eye = self.eye();
        let cell = eye.floor().as_ivec3();
        fluid(cell).and_then(|(kind, height)| (eye.y < cell.y as f32 + height).then_some(kind))
    }

    pub fn step(
        &mut self,
        input: &Input,
        yaw: f32,
        is_solid: impl Fn(IVec3) -> bool,
        fluid: impl Fn(IVec3) -> Option<(FluidKind, f32)>,
    ) {
        let collision = |pos| {
            if is_solid(pos) {
                Collision::FULL
            } else {
                Collision::NONE
            }
        };
        self.step_with(input, yaw, collision, fluid);
    }

    fn on_ice(&self, collision: &impl Fn(IVec3) -> Collision) -> bool {
        let below = IVec3::new(
            self.feet.x.floor() as i32,
            (self.feet.y - 2.0 * EPS).floor() as i32,
            self.feet.z.floor() as i32,
        );
        self.on_ground && collision(below).slippery
    }

    pub fn step_with(
        &mut self,
        input: &Input,
        yaw: f32,
        collision: impl Fn(IVec3) -> Collision,
        fluid: impl Fn(IVec3) -> Option<(FluidKind, f32)>,
    ) {
        let (sin, cos) = yaw.sin_cos();
        let forward = Vec3::new(cos, 0.0, sin);
        let right = Vec3::new(-sin, 0.0, cos);
        let wish = (forward * input.forward + right * input.strafe).clamp_length_max(1.0);
        let vertical = f32::from(u8::from(input.jump)) - f32::from(u8::from(input.descend));

        let immersion = match self.mode {
            Mode::Walking => self.fluid_depth(&fluid).filter(|&(_, depth)| depth > 0.0),
            Mode::Flying | Mode::Noclip => None,
        };
        let depth = immersion.map_or(0.0, |(_, depth)| depth);
        if input.forward <= 0.0 || depth > 0.0 {
            self.sprinting = false;
        }
        match self.mode {
            Mode::Walking if let Some((kind, _)) = immersion => {
                let medium = medium(kind);
                let target = Vec2::new(wish.x, wish.z) * medium.swim_speed;
                let horizontal = Vec2::new(self.velocity.x, self.velocity.z)
                    .move_towards(target, medium.accel * TICK);
                self.velocity.x = horizontal.x;
                self.velocity.z = horizontal.y;
                let mut vy = self.velocity.y - WATER_GRAVITY * TICK;
                if input.jump && depth > SWIM_DEPTH && vy < medium.swim_up_speed {
                    vy = (vy + medium.swim_up_accel * TICK).min(medium.swim_up_speed);
                }
                if vy < -medium.sink_speed {
                    vy = (vy * medium.drag).min(-medium.sink_speed);
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
                let current = Vec2::new(self.velocity.x, self.velocity.z);
                let horizontal = if self.on_ice(&collision) {
                    current.move_towards(target, ICE_ACCEL * TICK)
                } else if self.on_ground {
                    target
                } else {
                    current.move_towards(target, AIR_ACCEL * TICK)
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
        self.step_offset = (self.step_offset - TICK * STEP_HEIGHT / STEP_SMOOTH_TIME).max(0.0);
        let can_step = self.auto_step && self.mode == Mode::Walking && immersion.is_none();
        let mut against_wall = false;
        for axis in [1, 0, 2] {
            let before = self.feet;
            let mut blocked = self.move_axis(axis, delta[axis], &collision);
            if blocked && axis != 1 && can_step && self.on_ground {
                blocked = !self.step_up(before, axis, delta[axis], &collision);
            }
            against_wall |= blocked && axis != 1;
        }
        if against_wall {
            self.sprinting = false;
        }
        if let Some((kind, _)) = immersion
            && depth < WATER_EXIT_DEPTH
            && input.jump
            && against_wall
        {
            self.velocity.y = self.velocity.y.max(medium(kind).exit_speed);
        }
        if self.mode == Mode::Flying && self.on_ground {
            self.mode = Mode::Walking;
        }
    }

    fn step_up(
        &mut self,
        start: Vec3,
        axis: usize,
        delta: f32,
        collision: &impl Fn(IVec3) -> Collision,
    ) -> bool {
        let saved = (self.feet, self.velocity);
        self.feet = start;
        let raised = !self.move_axis(1, STEP_HEIGHT, collision)
            && !self.move_axis(axis, delta, collision)
            && {
                self.on_ground = false;
                self.move_axis(1, -STEP_HEIGHT, collision) && self.feet.y > start.y + EPS
            };
        if raised {
            self.step_offset += self.feet.y - start.y;
            return true;
        }
        (self.feet, self.velocity) = saved;
        self.on_ground = true;
        false
    }

    fn move_axis(
        &mut self,
        axis: usize,
        delta: f32,
        collision: &impl Fn(IVec3) -> Collision,
    ) -> bool {
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
        let (a, b) = match axis {
            0 => (1, 2),
            1 => (0, 2),
            _ => (0, 1),
        };
        let cells = |layer: i32| {
            let mut pos = lo;
            pos[axis] = layer;
            (lo[a]..=hi[a]).flat_map(move |i| {
                (lo[b]..=hi[b]).map(move |j| {
                    let mut cell = pos;
                    cell[a] = i;
                    cell[b] = j;
                    cell
                })
            })
        };
        let blocked = |layer: i32| {
            cells(layer).any(|cell| {
                let height = collision(cell).height;
                height > 0.0 && (axis == 1 || min.y + EPS < cell.y as f32 + height)
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
        } else if axis == 1 {
            let first = (min.y + EPS).floor() as i32;
            let last = (min.y + delta - EPS).floor() as i32;
            let floor = (last..=first)
                .flat_map(cells)
                .filter_map(|cell| {
                    let height = collision(cell).height;
                    (height > 0.0).then_some(cell.y as f32 + height)
                })
                .filter(|&top| top <= min.y + EPS && top >= min.y + delta - EPS)
                .reduce(f32::max);
            if let Some(top) = floor {
                self.feet.y = top;
                self.velocity.y = 0.0;
                self.on_ground = true;
                return true;
            }
        } else {
            let first = (min[axis] + EPS).floor() as i32 - 1;
            let last = (min[axis] + delta + EPS).floor() as i32;
            if let Some(layer) = (last..=first).rev().find(|&layer| blocked(layer)) {
                self.feet[axis] = (layer + 1) as f32 + below;
                self.velocity[axis] = 0.0;
                return true;
            }
        }
        self.feet[axis] += delta;
        false
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
