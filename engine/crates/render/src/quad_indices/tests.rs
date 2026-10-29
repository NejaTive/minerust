use super::*;

/// Углы квада для `vertex_index % 6` в прежнем шейдере без индексов.
const OLD_TRIANGLE_CORNERS: [u32; 6] = [0, 1, 2, 0, 2, 3];

#[test]
fn indices_cover_each_quad_with_four_vertices() {
    let indices = quad_indices(3);
    assert_eq!(indices.len(), 18);
    assert_eq!(&indices[..6], &[0, 1, 2, 0, 2, 3]);
    assert_eq!(&indices[12..], &[8, 9, 10, 8, 10, 11]);
    assert!(indices.iter().all(|&i| i < 12));
}

#[test]
fn indexed_path_gives_the_same_triangles_as_six_vertices_per_quad() {
    // Шейдер: старый путь — угол `(TRI[v % 6] + flip) & 3`, новый — `(v % 4 + flip) & 3`,
    // где v берётся из индекса.
    for flip in [0, 1] {
        let old: Vec<u32> = (0..6)
            .map(|v| (OLD_TRIANGLE_CORNERS[v] + flip) & 3)
            .collect();
        for q in [0u32, 1, 7] {
            let new: Vec<u32> = quad_indices(q + 1)[(6 * q) as usize..]
                .iter()
                .map(|&vertex| (vertex % 4 + flip) & 3)
                .collect();
            assert_eq!(new, old, "quad {q}, flip {flip}");
            let quad: Vec<u32> = quad_indices(q + 1)[(6 * q) as usize..]
                .iter()
                .map(|&vertex| vertex / 4)
                .collect();
            assert_eq!(quad, [q; 6]);
        }
    }
}
