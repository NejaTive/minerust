use std::collections::HashSet;

use glam::{IVec3, Vec3};

use crate::block::Block;
use crate::world::World;

pub const TNT_POWER: f32 = 4.0;

const RAY_GRID: i32 = 16;
const RAY_STEP: f32 = 0.3;
const RAY_DECAY: f32 = 0.225_000_01;
const RESISTANCE_BIAS: f32 = 0.3;
const DENSITY_STEP: f32 = 0.1;

/// Клетки, которые разрушит взрыв силы `power` в `center`; `random` отдаёт числа из `[0, 1)`.
pub fn destroyed_cells(
    world: &World,
    center: Vec3,
    power: f32,
    mut random: impl FnMut() -> f32,
) -> Vec<IVec3> {
    let mut seen = HashSet::new();
    let mut cells = Vec::new();
    let last = RAY_GRID - 1;
    for j in 0..RAY_GRID {
        for k in 0..RAY_GRID {
            for l in 0..RAY_GRID {
                if ![j, k, l].iter().any(|&i| i == 0 || i == last) {
                    continue;
                }
                let dir = Vec3::new(j as f32, k as f32, l as f32) / last as f32 * 2.0 - Vec3::ONE;
                let dir = dir.normalize() * RAY_STEP;
                let mut energy = power * (0.7 + random() * 0.6);
                let mut at = center;
                while energy > 0.0 {
                    let pos = at.floor().as_ivec3();
                    let block = world.block(pos);
                    if block != Block::Air {
                        energy -= (block.blast_resistance() + RESISTANCE_BIAS) * RAY_STEP;
                    }
                    if energy > 0.0 && block != Block::Air && seen.insert(pos) {
                        cells.push(pos);
                    }
                    energy -= RAY_DECAY;
                    at += dir;
                }
            }
        }
    }
    cells
}

pub struct Destruction {
    pub changed: Vec<IVec3>,
    pub tnt: Vec<IVec3>,
}

pub fn destroy(world: &mut World, cells: &[IVec3]) -> Destruction {
    let mut changed = Vec::new();
    let mut tnt = Vec::new();
    for &pos in cells {
        if world.block(pos) == Block::Tnt {
            tnt.push(pos);
        }
        changed.extend(world.set_block_supported(pos, Block::Air));
    }
    Destruction { changed, tnt }
}

/// Доля тела, видимая из центра взрыва (`World.getBlockDensity`); лучи глохнут о твёрдые блоки.
pub fn exposure(world: &World, center: Vec3, (min, max): (Vec3, Vec3)) -> f32 {
    let size = max - min;
    let step = Vec3::ONE / (size * 2.0 + Vec3::ONE);
    let margin_x = (1.0 - (1.0 / step.x).floor() * step.x) / 2.0;
    let margin_z = (1.0 - (1.0 / step.z).floor() * step.z) / 2.0;
    let (mut seen, mut total) = (0u32, 0u32);
    let mut fx = 0.0;
    while fx <= 1.0 {
        let mut fy = 0.0;
        while fy <= 1.0 {
            let mut fz = 0.0;
            while fz <= 1.0 {
                let point = Vec3::new(
                    min.x + size.x * fx + margin_x,
                    min.y + size.y * fy,
                    min.z + size.z * fz + margin_z,
                );
                if sees(world, center, point) {
                    seen += 1;
                }
                total += 1;
                fz += step.z;
            }
            fy += step.y;
        }
        fx += step.x;
    }
    seen as f32 / total as f32
}

fn sees(world: &World, from: Vec3, to: Vec3) -> bool {
    let delta = to - from;
    let steps = (delta.length() / DENSITY_STEP).ceil().max(1.0) as u32;
    (1..steps).all(|i| {
        let at = from + delta * (i as f32 / steps as f32);
        !world.block(at.floor().as_ivec3()).is_solid()
    })
}

/// Приращение скорости тела от взрыва в блоках за тик 1.12; `eye` — высота центра отсчёта над `feet`.
pub fn knockback(world: &World, center: Vec3, power: f32, body: (Vec3, Vec3), eye: f32) -> Vec3 {
    let diameter = power * 2.0;
    let feet = Vec3::new(
        (body.0.x + body.1.x) / 2.0,
        body.0.y,
        (body.0.z + body.1.z) / 2.0,
    );
    let distance = feet.distance(center) / diameter;
    if distance > 1.0 {
        return Vec3::ZERO;
    }
    let offset = Vec3::new(feet.x, feet.y + eye, feet.z) - center;
    if offset.length() == 0.0 {
        return Vec3::ZERO;
    }
    let strength = (1.0 - distance) * exposure(world, center, body);
    offset.normalize() * strength
}

#[cfg(test)]
mod tests;
