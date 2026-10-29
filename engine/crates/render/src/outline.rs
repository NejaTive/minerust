use glam::{IVec3, Vec3};

const INFLATE: f32 = 0.002;

pub const OUTLINE_VERTICES: usize = 24;
pub const OUTLINE_VERTEX_SIZE: usize = 12;

pub fn outline_vertices(block: IVec3, bounds: (Vec3, Vec3)) -> [Vec3; OUTLINE_VERTICES] {
    let min = block.as_vec3() + bounds.0 - INFLATE;
    let size = bounds.1 - bounds.0 + 2.0 * INFLATE;
    let corner = |i: usize| {
        min + Vec3::new(
            (i & 1) as f32 * size.x,
            (i >> 1 & 1) as f32 * size.y,
            (i >> 2 & 1) as f32 * size.z,
        )
    };
    let mut vertices = [Vec3::ZERO; OUTLINE_VERTICES];
    let edges = (0..8usize).flat_map(|i| {
        (0..3)
            .filter(move |bit| i & (1 << bit) == 0)
            .map(move |bit| (i, i | (1 << bit)))
    });
    for (n, (a, b)) in edges.enumerate() {
        vertices[2 * n] = corner(a);
        vertices[2 * n + 1] = corner(b);
    }
    vertices
}

pub fn outline_bytes(block: IVec3, bounds: (Vec3, Vec3)) -> Vec<u8> {
    outline_vertices(block, bounds)
        .iter()
        .flat_map(|v| v.to_array())
        .flat_map(f32::to_le_bytes)
        .collect()
}

#[cfg(test)]
mod tests;
