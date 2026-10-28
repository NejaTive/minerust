use super::*;

#[test]
fn vertex_is_eight_bytes() {
    let v = Vertex::new([1, 2, 3], [0, 16], 128, 7, [0, 0]);
    let bytes = vertex_bytes(&[v, v]);
    assert_eq!(size_of::<Vertex>(), 8);
    assert_eq!(bytes.len(), 2 * 8);
    let word0 = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    assert_eq!(word0 & 0x7FF_FFFF, 1 | 2 << 9 | 3 << 18);
}

#[test]
fn position_bounds_round_trip() {
    for p in [0, 1, 14, 16, 255, MAX_POSITION] {
        let v = Vertex::new([p, MAX_POSITION - p, p], [0, 0], 0, 0, [0, 0]);
        let [x, y, z] = v.position();
        assert_eq!(x, f32::from(p) / 16.0);
        assert_eq!(y, f32::from(MAX_POSITION - p) / 16.0);
        assert_eq!(z, x);
    }
    let max = Vertex::new([MAX_POSITION; 3], [16, 16], 255, 255, [0, 0]);
    assert_eq!(max.position(), [16.0; 3]);
    assert_eq!(max.uv(), [1.0, 1.0]);
}

#[test]
fn water_surface_is_exact() {
    let v = Vertex::new([0, 14, 0], [0, 2], 0, 0, [0, 0]);
    assert_eq!(v.position()[1], world::block::WATER_SURFACE);
    assert_eq!(v.uv()[1], 1.0 - world::block::WATER_SURFACE);
}

#[test]
fn fields_do_not_overlap() {
    let v = Vertex::new([MAX_POSITION; 3], [16, 16], 0, 255, [0, 0]);
    assert_eq!((v.layer(), v.shade_byte()), (255, 0));
    assert_eq!((v.position(), v.uv()), ([16.0; 3], [1.0; 2]));
    let v = Vertex::new([0; 3], [0; 2], 255, 0, [0, 0]);
    assert_eq!(
        (v.layer(), v.shade_byte(), v.position(), v.uv()),
        (0, 255, [0.0; 3], [0.0; 2])
    );
    // Свет не залезает в соседние поля и наоборот; бит 31 остаётся нулевым.
    let v = Vertex::new([MAX_POSITION; 3], [16, 16], 255, 255, [MAX_LIGHT_HALVES; 2]);
    assert_eq!((v.sun_halves(), v.block_halves()), (30, 30));
    assert_eq!((v.layer(), v.shade_byte()), (255, 255));
    assert_eq!((v.position(), v.uv()), ([16.0; 3], [1.0; 2]));
    assert_eq!(v.0[1] >> 31, 0);
    let v = Vertex::new([0; 3], [0; 2], 0, 0, [MAX_LIGHT_HALVES, 0]);
    assert_eq!(
        (v.sun_halves(), v.block_halves(), v.0[0], v.0[1] & 0x1FFFFF),
        (30, 0, 0, 0)
    );
    let v = Vertex::new([0; 3], [0; 2], 0, 0, [0, MAX_LIGHT_HALVES]);
    assert_eq!((v.sun_halves(), v.block_halves()), (0, 30));
}

#[test]
fn shade_bounds() {
    assert_eq!(quantize_shade(0.0), 0);
    assert_eq!(quantize_shade(1.0), 255);
    assert_eq!(quantize_shade(1.5), 255);
    assert_eq!(Vertex::new([0; 3], [0; 2], 0, 0, [0, 0]).shade(), 0.0);
    assert_eq!(Vertex::new([0; 3], [0; 2], 255, 0, [0, 0]).shade(), 1.0);
    // Ошибка квантования не больше половины шага.
    for s in [0.05, 0.3, 0.5, 0.8, 0.999] {
        let back = Vertex::new([0; 3], [0; 2], quantize_shade(s), 0, [0, 0]).shade();
        assert!((back - s).abs() <= 0.5 / 255.0 + 1e-6, "{s} -> {back}");
    }
}

#[test]
fn origin_bytes_hold_three_floats() {
    let bytes = origin_bytes([-32.0, 16.0, 48.0]);
    assert_eq!(&bytes[0..4], &(-32.0f32).to_le_bytes());
    assert_eq!(&bytes[8..12], &48.0f32.to_le_bytes());
    assert_eq!(&bytes[12..], &[0; 4]);
}

#[test]
fn flipped_quad_keeps_triangles() {
    let quad: Vec<Vertex> = (0..4)
        .map(|i| Vertex::new([i, 0, 0], [0, 0], 0, 0, [0, 0]))
        .collect();
    let straight = gpu_vertices(&quad, &[0, 1, 2, 0, 2, 3]);
    assert_eq!(straight, quad);
    let flipped = gpu_vertices(&quad, &[1, 2, 3, 1, 3, 0]);
    // Общие индексы 0,1,2 и 0,2,3 над сдвинутыми вершинами — те же две тройки.
    let xs = |at: [usize; 3]| at.map(|i| flipped[i].position()[0] * 16.0);
    assert_eq!(xs([0, 1, 2]), [1.0, 2.0, 3.0]);
    assert_eq!(xs([0, 2, 3]), [1.0, 3.0, 0.0]);
}

#[test]
fn quad_indices_cover_every_quad() {
    let bytes = quad_index_bytes();
    assert_eq!(bytes.len(), MAX_QUADS * 6 * 2);
    let last = u16::from_le_bytes(bytes[bytes.len() - 2..].try_into().unwrap());
    assert_eq!(usize::from(last), MAX_QUADS * 4 - 1);
}

#[test]
fn light_halves_round_trip() {
    for sun in [0, 1, 14, 15, 29, MAX_LIGHT_HALVES] {
        for block in [0, 1, 15, 30] {
            let v = Vertex::new([3; 3], [1, 2], 77, 9, [sun, block]);
            assert_eq!((v.sun_halves(), v.block_halves()), (sun, block));
            assert_eq!((v.shade_byte(), v.layer()), (77, 9));
        }
    }
}

/// Формула шейдера (`vertex_brightness` в shader.wgsl) повторена здесь по тексту:
/// `pow(0.8, 15 − level)` через `exp2`, минимум 0.05.
fn shader_brightness(sun_halves: u8, block_halves: u8, dark: f32) -> f32 {
    let level = (f32::from(sun_halves) / 2.0 - dark).max(f32::from(block_halves) / 2.0);
    (0.8f32.log2() * (15.0 - level)).exp2().max(0.05)
}

#[test]
fn shader_formula_matches_cpu() {
    for sun in 0..=MAX_LIGHT_HALVES {
        for block in [0, 7, 20, 30] {
            for dark in [0.0, 0.5, 5.5, 11.0] {
                let (cpu, gpu) = (
                    light_brightness(sun, block, dark),
                    shader_brightness(sun, block, dark),
                );
                assert!(
                    (cpu - gpu).abs() < 1e-5,
                    "{sun} {block} {dark}: {cpu} {gpu}"
                );
            }
        }
    }
    // Целые уровни при тьме 0 — ровно прежняя `light::brightness`.
    for level in 0..=MAX_LIGHT {
        assert_eq!(
            light_brightness(level * 2, 0, 0.0),
            world::light::brightness(level)
        );
    }
}

#[test]
fn darkness_leaves_block_light() {
    let v = Vertex::new([0; 3], [0; 2], 255, 0, [30, 20]);
    assert_eq!(v.brightness(0.0), 1.0);
    // Ночью солнце 15 − 11 = 4 слабее светокамня 10.
    assert_eq!(v.brightness(11.0), level_brightness(10.0));
    let sun_only = Vertex::new([0; 3], [0; 2], 255, 0, [30, 0]);
    assert!(sun_only.brightness(11.0) > 0.05);
    assert_eq!(sun_only.brightness(11.0), level_brightness(4.0));
    assert_eq!(
        Vertex::new([0; 3], [0; 2], 255, 0, [0, 0]).brightness(11.0),
        0.05
    );
}

#[test]
fn day_shade_matches_vertex() {
    for (shade, sun, block) in [(255, 30, 0), (200, 12, 25), (128, 0, 0), (0, 30, 30)] {
        let v = Vertex::new([0; 3], [0; 2], shade, 0, [sun, block]);
        assert_eq!(
            day_shade(shade, sun, block),
            quantize_shade(v.brightness(0.0))
        );
    }
}
