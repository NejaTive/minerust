use glam::{Mat4, Vec3, Vec4};

pub struct Frustum {
    planes: [Vec4; 6],
}

impl Frustum {
    pub fn from_view_proj(m: Mat4) -> Self {
        let [r0, r1, r2, r3] = [m.row(0), m.row(1), m.row(2), m.row(3)];
        Self {
            planes: [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2],
        }
    }

    pub fn intersects_aabb(&self, min: Vec3, max: Vec3) -> bool {
        self.planes.iter().all(|plane| {
            let normal = plane.truncate();
            let p = Vec3::select(normal.cmpge(Vec3::ZERO), max, min);
            normal.dot(p) + plane.w >= 0.0
        })
    }
}

#[cfg(test)]
mod tests;
