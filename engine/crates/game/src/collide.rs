use glam::{Vec2, Vec3};

pub const PUSH_PER_TICK: f32 = 0.04;
const MIN_PUSH_SHARE: f32 = 0.25;
const SAME_SPOT: f32 = 0.01;

pub type Aabb = (Vec3, Vec3);

pub fn center_xz(bounds: Aabb) -> Vec2 {
    let mid = (bounds.0 + bounds.1) / 2.0;
    Vec2::new(mid.x, mid.z)
}

pub fn overlap_depth(a: Aabb, b: Aabb) -> Option<f32> {
    let span = (a.1.min(b.1) - a.0.max(b.0)).to_array();
    if span.iter().any(|s| *s <= 0.0) {
        return None;
    }
    let width = (a.1.x - a.0.x).min(b.1.x - b.0.x);
    Some(span[0].min(span[2]) / width)
}

/// Смещение второго тела от первого за тик; `tie` разводит тела, чьи центры совпали.
pub fn push_away(a: Aabb, b: Aabb, depth: f32, tie: u32) -> Vec2 {
    let delta = center_xz(b) - center_xz(a);
    let direction = if delta.length() < SAME_SPOT {
        Vec2::from_angle(tie as f32 * 2.399_963)
    } else {
        delta.normalize()
    };
    direction * PUSH_PER_TICK * depth.clamp(MIN_PUSH_SHARE, 1.0)
}

#[cfg(test)]
mod tests;
