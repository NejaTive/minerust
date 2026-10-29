use super::*;

#[test]
fn quad_is_twelve_bytes() {
    let q = Quad::new([1, 2, 3], 24, true, 7, [0, 1, 2, 3], [0; 4], [0; 4]);
    let bytes = quad_bytes(&[q, q]);
    assert_eq!(size_of::<Quad>(), 12);
    assert_eq!(bytes.len(), 2 * 12);
    let word0 = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    assert_eq!(word0, 1 | 2 << 4 | 3 << 8 | 24 << 12 | 1 << 17 | 7 << 18);
    let word1 = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    assert_eq!(word1, 0b11_10_01_00);
}

#[test]
fn quad_fields_round_trip_at_bounds() {
    let q = Quad::new(
        [15, 0, 15],
        (crate::templates::TEMPLATE_COUNT - 1) as u8,
        false,
        255,
        [3, 0, 3, 1],
        [MAX_LIGHT_HALVES, 0, 1, 29],
        [0, MAX_LIGHT_HALVES, 17, 2],
    );
    assert_eq!(q.cell(), [15, 0, 15]);
    assert_eq!(
        usize::from(q.template()),
        crate::templates::TEMPLATE_COUNT - 1
    );
    assert!(!q.flipped());
    assert_eq!(q.layer(), 255);
    assert_eq!(q.ao(), [3, 0, 3, 1]);
    assert_eq!(q.sun_halves(), [MAX_LIGHT_HALVES, 0, 1, 29]);
    assert_eq!(q.block_halves(), [0, MAX_LIGHT_HALVES, 17, 2]);
    let flipped = Quad::new([15, 0, 15], 24, true, 0, [0; 4], [0; 4], [0; 4]);
    assert_eq!(
        (flipped.template(), flipped.layer(), flipped.cell()),
        (24, 0, [15, 0, 15])
    );
}

#[test]
fn shade_bounds() {
    assert_eq!(quantize_shade(0.0), 0);
    assert_eq!(quantize_shade(1.0), 255);
    assert_eq!(quantize_shade(1.5), 255);
}

#[test]
fn slot_bytes_hold_origin_and_first_word() {
    let bytes = slot_bytes([-32.0, 16.0, 48.0], 0x0102_0304);
    assert_eq!(&bytes[0..4], &(-32.0f32).to_le_bytes());
    assert_eq!(&bytes[8..12], &48.0f32.to_le_bytes());
    assert_eq!(&bytes[12..], &0x0102_0304u32.to_le_bytes());
}

/// Формула шейдера (`vertex_brightness` в shader.wgsl) повторена здесь по тексту:
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
    for level in 0..=MAX_LIGHT {
        assert_eq!(
            light_brightness(level * 2, 0, 0.0),
            world::light::brightness(level)
        );
    }
}

#[test]
fn darkness_leaves_block_light() {
    assert_eq!(light_brightness(30, 20, 0.0), 1.0);
    assert_eq!(light_brightness(30, 20, 11.0), level_brightness(10.0));
    assert!(light_brightness(30, 0, 11.0) > 0.05);
    assert_eq!(light_brightness(30, 0, 11.0), level_brightness(4.0));
    assert_eq!(light_brightness(0, 0, 11.0), 0.05);
}

#[test]
fn day_shade_is_quantized_shade_times_light() {
    for (shade, sun, block) in [(255, 30, 0), (200, 12, 25), (128, 0, 0), (0, 30, 30)] {
        assert_eq!(
            day_shade(shade, sun, block),
            quantize_shade(f32::from(shade) / 255.0 * light_brightness(sun, block, 0.0))
        );
    }
}
