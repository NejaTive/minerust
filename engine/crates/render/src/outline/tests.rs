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
