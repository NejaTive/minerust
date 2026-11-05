use std::f32::consts::TAU;

use glam::{IVec3, Vec3};

use crate::physics::Collision;
use crate::rng::Rng;

/// Тик 1.12 в тиках физики (60 Гц против 20 Гц).
pub const MC_TICK: u32 = 3;
pub const FUSE_TICKS: i32 = 80;
pub const SIZE: f32 = 0.98;
const GRAVITY: f32 = 0.04;
const DRAG: f32 = 0.98;
const GROUND_FRICTION: f32 = 0.7;
const BOUNCE: f32 = -0.5;
const KICK_UP: f32 = 0.2;
const KICK_SIDE: f32 = 0.02;
const EPS: f32 = 1e-4;

pub const FLASH_TICKS: i32 = 5;
const SWELL_TICKS: f32 = 10.0;
const SWELL_SIZE: f32 = 0.3;

/// Масштаб куба при `left` тиках до взрыва: раздувание последних десяти тиков, как в `RenderTNTPrimed`.
pub fn swell(left: f32) -> f32 {
    if left >= SWELL_TICKS {
        return 1.0;
    }
    let f = (1.0 - left / SWELL_TICKS).clamp(0.0, 1.0);
    1.0 + f.powi(4) * SWELL_SIZE
}

/// Центр взрыва выше низа сущности на `height / 16`.
pub const BLAST_LIFT: f32 = SIZE / 16.0;

#[derive(Clone, Debug)]
pub struct PrimedTnt {
    pub pos: Vec3,
    pub prev: Vec3,
    /// Блоков за тик 1.12.
    pub velocity: Vec3,
    pub fuse: i32,
    on_ground: bool,
}

impl PrimedTnt {
    pub fn at(cell: IVec3, fuse: i32, rng: &mut Rng) -> Self {
        let angle = rng.unit() * TAU;
        let pos = Vec3::new(cell.x as f32 + 0.5, cell.y as f32, cell.z as f32 + 0.5);
        Self {
            pos,
            prev: pos,
            velocity: Vec3::new(-angle.sin() * KICK_SIDE, KICK_UP, -angle.cos() * KICK_SIDE),
            fuse,
            on_ground: false,
        }
    }

    pub fn aabb(&self) -> (Vec3, Vec3) {
        let half = SIZE / 2.0;
        (
            self.pos - Vec3::new(half, 0.0, half),
            self.pos + Vec3::new(half, SIZE, half),
        )
    }

    pub fn blast_center(&self) -> Vec3 {
        self.pos + Vec3::Y * BLAST_LIFT
    }

    /// Один тик 1.12; `true`, когда запал догорел.
    pub fn tick(&mut self, collision: impl Fn(IVec3) -> Collision) -> bool {
        self.prev = self.pos;
        self.velocity.y -= GRAVITY;
        let wanted = self.velocity;
        for axis in [1, 0, 2] {
            let moved = self.move_axis(axis, wanted[axis], &collision);
            if moved != wanted[axis] {
                self.velocity[axis] = 0.0;
            }
        }
        self.on_ground = wanted.y < 0.0 && self.velocity.y == 0.0 && wanted.y != 0.0;
        self.velocity *= DRAG;
        if self.on_ground {
            self.velocity.x *= GROUND_FRICTION;
            self.velocity.z *= GROUND_FRICTION;
            self.velocity.y *= BOUNCE;
        }
        self.fuse -= 1;
        self.fuse <= 0
    }

    fn move_axis(
        &mut self,
        axis: usize,
        delta: f32,
        collision: &impl Fn(IVec3) -> Collision,
    ) -> f32 {
        if delta == 0.0 {
            return 0.0;
        }
        let (min, max) = self.aabb();
        let lo = (min + Vec3::splat(EPS)).min(min + Vec3::splat(EPS) + axis_vec(axis, delta));
        let hi = (max - Vec3::splat(EPS)).max(max - Vec3::splat(EPS) + axis_vec(axis, delta));
        let (lo, hi) = (lo.floor().as_ivec3(), hi.floor().as_ivec3());
        let mut allowed = delta;
        for y in lo.y..=hi.y {
            for z in lo.z..=hi.z {
                for x in lo.x..=hi.x {
                    let cell = IVec3::new(x, y, z);
                    collision(cell).for_each_box(cell, |bmin, bmax| {
                        let others = [0, 1, 2].into_iter().filter(|&a| a != axis);
                        let overlaps = others
                            .into_iter()
                            .all(|a| min[a] + EPS < bmax[a] && max[a] - EPS > bmin[a]);
                        if !overlaps {
                            return;
                        }
                        if allowed > 0.0 && bmin[axis] >= max[axis] - EPS {
                            allowed = allowed.min(bmin[axis] - max[axis]);
                        } else if allowed < 0.0 && bmax[axis] <= min[axis] + EPS {
                            allowed = allowed.max(bmax[axis] - min[axis]);
                        }
                    });
                }
            }
        }
        self.pos[axis] += allowed;
        allowed
    }
}

fn axis_vec(axis: usize, value: f32) -> Vec3 {
    let mut v = Vec3::ZERO;
    v[axis] = value;
    v
}

#[cfg(test)]
mod tests;
