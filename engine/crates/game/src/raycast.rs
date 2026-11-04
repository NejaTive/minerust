use glam::{IVec3, Vec3};
use world::boxes::BoxList;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub block: IVec3,
    pub normal: IVec3,
    pub distance: f32,
    /// Коробка внутри клетки, в долях блока; `None` — вся клетка.
    pub bounds: Option<(Vec3, Vec3)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Probe {
    Miss,
    Full,
    Boxes(BoxList),
}

pub fn raycast(
    origin: Vec3,
    dir: Vec3,
    max_distance: f32,
    is_solid: impl Fn(IVec3) -> bool,
) -> Option<Hit> {
    raycast_shapes(origin, dir, max_distance, |pos| {
        if is_solid(pos) {
            Probe::Full
        } else {
            Probe::Miss
        }
    })
}

fn box_entry(origin: Vec3, dir: Vec3, min: Vec3, max: Vec3) -> Option<(f32, usize)> {
    let (mut enter, mut exit, mut axis) = (f32::NEG_INFINITY, f32::INFINITY, 0);
    for k in 0..3 {
        if dir[k] == 0.0 {
            if origin[k] < min[k] || origin[k] > max[k] {
                return None;
            }
            continue;
        }
        let (a, b) = ((min[k] - origin[k]) / dir[k], (max[k] - origin[k]) / dir[k]);
        let (near, far) = (a.min(b), a.max(b));
        if near > enter {
            enter = near;
            axis = k;
        }
        exit = exit.min(far);
    }
    (enter >= 0.0 && enter <= exit).then_some((enter, axis))
}

pub fn raycast_shapes(
    origin: Vec3,
    dir: Vec3,
    max_distance: f32,
    probe: impl Fn(IVec3) -> Probe,
) -> Option<Hit> {
    let dir = dir.normalize_or_zero();
    if dir == Vec3::ZERO || !origin.is_finite() || max_distance.is_nan() {
        return None;
    }
    let mut voxel = origin.floor().as_ivec3();
    let step = IVec3::new(sign(dir.x), sign(dir.y), sign(dir.z));
    let t_delta = dir.abs().recip();
    let mut t_max = Vec3::from_array(std::array::from_fn(|axis| {
        if dir[axis] == 0.0 {
            return f32::INFINITY;
        }
        let to_border = if dir[axis] > 0.0 {
            voxel[axis] as f32 + 1.0 - origin[axis]
        } else {
            origin[axis] - voxel[axis] as f32
        };
        to_border * t_delta[axis]
    }));

    loop {
        let axis = if t_max.x <= t_max.y && t_max.x <= t_max.z {
            0
        } else if t_max.y <= t_max.z {
            1
        } else {
            2
        };
        let distance = t_max[axis];
        if distance > max_distance {
            return None;
        }
        voxel[axis] += step[axis];
        t_max[axis] += t_delta[axis];
        match probe(voxel) {
            Probe::Miss => {}
            Probe::Full => {
                let mut normal = IVec3::ZERO;
                normal[axis] = -step[axis];
                return Some(Hit {
                    block: voxel,
                    normal,
                    distance,
                    bounds: None,
                });
            }
            Probe::Boxes(list) => {
                let local = origin - voxel.as_vec3();
                let nearest = list
                    .iter()
                    .filter(|b| !b.is_flat())
                    .filter_map(|b| {
                        let (min, max) = b.bounds();
                        let (t, hit_axis) = box_entry(local, dir, min, max)?;
                        Some((t, hit_axis, (min, max)))
                    })
                    .filter(|(t, _, _)| *t <= max_distance)
                    .min_by(|a, b| a.0.total_cmp(&b.0));
                if let Some((t, hit_axis, bounds)) = nearest {
                    let mut normal = IVec3::ZERO;
                    normal[hit_axis] = if dir[hit_axis] > 0.0 { -1 } else { 1 };
                    return Some(Hit {
                        block: voxel,
                        normal,
                        distance: t,
                        bounds: Some(bounds),
                    });
                }
            }
        }
    }
}

fn sign(v: f32) -> i32 {
    if v > 0.0 {
        1
    } else if v < 0.0 {
        -1
    } else {
        0
    }
}

#[cfg(test)]
mod tests;
