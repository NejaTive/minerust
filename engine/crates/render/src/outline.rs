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
mod tests {
    use super::*;

    #[test]
    fn twelve_axis_aligned_edges_around_block() {
        let block = IVec3::new(-3, 70, 5);
        let vertices = outline_vertices(block, (Vec3::ZERO, Vec3::ONE));
        let mut axes = [0; 3];
        let mut edges: Vec<[i32; 6]> = Vec::new();
        for pair in vertices.chunks(2) {
            let d = (pair[1] - pair[0]).abs();
            let axis = d.max_position();
            assert!((d[axis] - (1.0 + 2.0 * INFLATE)).abs() < 1e-5, "{d}");
            assert!(
                d.element_sum() - d[axis] < 1e-6,
                "edge not axis-aligned: {d}"
            );
            axes[axis] += 1;
            for v in pair {
                // Каждый конец — угол блока, раздутый наружу.
                let off = *v - block.as_vec3();
                assert!(off.cmpge(Vec3::splat(-INFLATE - 1e-5)).all());
                assert!(off.cmple(Vec3::splat(1.0 + INFLATE + 1e-5)).all());
            }
            let key = |v: Vec3| (v * 1000.0).round().as_ivec3().to_array();
            let (a, b) = (key(pair[0]), key(pair[1]));
            edges.push([a[0], a[1], a[2], b[0], b[1], b[2]]);
        }
        assert_eq!(axes, [4, 4, 4]);
        edges.sort();
        edges.dedup();
        assert_eq!(edges.len(), 12);
        assert_eq!(
            outline_bytes(block, (Vec3::ZERO, Vec3::ONE)).len(),
            OUTLINE_VERTICES * OUTLINE_VERTEX_SIZE
        );
    }

    #[test]
    fn box_outline_follows_bounds() {
        let block = IVec3::new(2, 5, -4);
        let bounds = (Vec3::new(0.25, 0.0, 0.25), Vec3::new(0.5, 0.625, 0.5));
        let vertices = outline_vertices(block, bounds);
        let min = vertices.iter().fold(Vec3::MAX, |a, &v| a.min(v));
        let max = vertices.iter().fold(Vec3::MIN, |a, &v| a.max(v));
        let origin = block.as_vec3();
        assert!((min - (origin + bounds.0 - INFLATE)).abs().max_element() < 1e-5);
        assert!((max - (origin + bounds.1 + INFLATE)).abs().max_element() < 1e-5);
    }
}
