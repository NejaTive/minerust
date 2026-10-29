use glam::{IVec3, Vec3};

/// Насколько рамка больше блока с каждой стороны, блоков. Линии ровно на гранях
/// мерцали бы в z-fighting с самими гранями.
const INFLATE: f32 = 0.002;

/// 12 рёбер куба парами концов для `LineList`.
pub const OUTLINE_VERTICES: usize = 24;
/// Вершина рамки — только позиция, `Float32x3`.
pub const OUTLINE_VERTEX_SIZE: usize = 12;

/// Рёбра рамки вокруг бокса `bounds` (min, max в долях клетки) блока `block`, в
/// мировых координатах, парами концов.
pub fn outline_vertices(block: IVec3, bounds: (Vec3, Vec3)) -> [Vec3; OUTLINE_VERTICES] {
    let min = block.as_vec3() + bounds.0 - INFLATE;
    let size = bounds.1 - bounds.0 + 2.0 * INFLATE;
    // Угол куба по номеру 0..8: бит 0 — x, бит 1 — y, бит 2 — z.
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
