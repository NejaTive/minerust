use glam::{IVec3, Vec2, Vec3};

const MAX_DROP: f32 = 2.0;
const MAX_OPACITY: f32 = 0.5;
const LIFT: f32 = 0.02;
const BELOW_FEET: f32 = 1e-3;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Ground {
    Air,
    Blocked,
    Solid(f32),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ShadowPatch {
    pub min: Vec2,
    pub max: Vec2,
    pub y: f32,
    pub opacity: f32,
}

pub fn opacity_at(drop: f32) -> f32 {
    MAX_OPACITY * (1.0 - drop / MAX_DROP).clamp(0.0, 1.0)
}

pub fn shadow_patches(
    feet: Vec3,
    radius: f32,
    ground: impl Fn(IVec3) -> Ground,
) -> Vec<ShadowPatch> {
    let (lo, hi) = (
        Vec2::new(feet.x - radius, feet.z - radius),
        Vec2::new(feet.x + radius, feet.z + radius),
    );
    let top_cell = (feet.y - BELOW_FEET).floor() as i32;
    let bottom_cell = (feet.y - MAX_DROP).floor() as i32;
    let mut patches = Vec::new();
    for x in lo.x.floor() as i32..=hi.x.floor() as i32 {
        for z in lo.y.floor() as i32..=hi.y.floor() as i32 {
            let top =
                (bottom_cell..=top_cell)
                    .rev()
                    .find_map(|y| match ground(IVec3::new(x, y, z)) {
                        Ground::Air => None,
                        Ground::Blocked => Some(None),
                        Ground::Solid(height) => Some(Some(y as f32 + height)),
                    });
            let Some(Some(top)) = top else { continue };
            let drop = feet.y - top;
            if !(-BELOW_FEET..=MAX_DROP).contains(&drop) {
                continue;
            }
            let min = Vec2::new((x as f32).max(lo.x), (z as f32).max(lo.y));
            let max = Vec2::new((x as f32 + 1.0).min(hi.x), (z as f32 + 1.0).min(hi.y));
            if min.x < max.x && min.y < max.y {
                patches.push(ShadowPatch {
                    min,
                    max,
                    y: top + LIFT,
                    opacity: opacity_at(drop.max(0.0)),
                });
            }
        }
    }
    patches
}

#[cfg(test)]
mod tests;
