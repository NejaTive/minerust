use super::*;

fn instance() -> MobInstance {
    MobInstance {
        kind: ModelKind::Pig,
        feet: Vec3::new(10.0, 64.0, -5.0),
        yaw: 0.0,
        head_yaw: 0.0,
        head_pitch: 0.0,
        limb_phase: 0.0,
        limb_swing: 0.0,
        sky_level: 15.0,
        block_level: 0.0,
        shadow_radius: 0.7,
        shadow: vec![ShadowPatch {
            min: Vec2::new(9.5, -5.5),
            max: Vec2::new(10.5, -4.5),
            y: 64.02,
            opacity: 0.5,
        }],
    }
}

fn vertices(instance: &MobInstance) -> Vec<[f32; VERTEX_FLOATS]> {
    let mut out = Vec::new();
    push_mob(instance, &mut out);
    out.as_chunks::<VERTEX_FLOATS>().0.to_vec()
}

fn position(v: &[f32; VERTEX_FLOATS]) -> Vec3 {
    Vec3::new(v[0], v[1], v[2])
}

#[test]
fn pig_has_seven_boxes() {
    assert_eq!(vertices(&instance()).len(), 7 * 6 * 6);
}

#[test]
fn standing_pig_matches_the_hitbox_footprint() {
    let pig = instance();
    let points: Vec<Vec3> = vertices(&pig).iter().map(position).collect();
    let min = points.iter().copied().reduce(Vec3::min).unwrap();
    let max = points.iter().copied().reduce(Vec3::max).unwrap();
    assert!((min.y - pig.feet.y).abs() < 1e-4, "{min}");
    assert!((max.y - pig.feet.y - 1.0).abs() < 0.1, "{max}");
    assert!((max.x - min.x - 1.5).abs() < 0.02, "{min} {max}");
    assert!((max.z - min.z - 0.625).abs() < 0.02, "{min} {max}");
}

#[test]
fn head_points_along_the_yaw() {
    for yaw in [0.0, 1.0, -2.0, 3.0] {
        let pig = MobInstance { yaw, ..instance() };
        let forward = Vec3::new(yaw.cos(), 0.0, yaw.sin());
        let snout = vertices(&pig)
            .iter()
            .map(position)
            .max_by(|a, b| {
                (*a - pig.feet)
                    .dot(forward)
                    .total_cmp(&(*b - pig.feet).dot(forward))
            })
            .unwrap();
        let offset = snout - pig.feet;
        assert!(offset.dot(forward) > 0.5, "yaw {yaw}: {offset}");
        assert!(offset.y > 0.3, "yaw {yaw}: snout height {offset}");
    }
}

#[test]
fn triangles_wind_counter_clockwise_outward() {
    let pig = instance();
    let verts = vertices(&pig);
    let center = verts.iter().map(position).sum::<Vec3>() / verts.len() as f32;
    for tri in verts.as_chunks::<3>().0 {
        let [a, b, c] = [position(&tri[0]), position(&tri[1]), position(&tri[2])];
        let normal = (b - a).cross(c - a);
        assert!(normal.length() > 0.0);
        let facing = normal.dot((a + b + c) / 3.0 - center);
        assert!(facing > -0.05, "{facing}");
    }
}

#[test]
fn texture_coordinates_stay_on_the_sheet() {
    for v in vertices(&instance()) {
        assert!(
            (0.0..=1.0).contains(&v[3]) && (0.0..=1.0).contains(&v[4]),
            "{v:?}"
        );
    }
}

#[test]
fn legs_swing_in_opposite_pairs() {
    let walking = MobInstance {
        limb_phase: 1.0,
        limb_swing: 1.0,
        ..instance()
    };
    let feet =
        |v: Vec<[f32; VERTEX_FLOATS]>| -> Vec<Vec3> { v[3 * 36..].iter().map(position).collect() };
    let still = feet(vertices(&instance()));
    let moving = feet(vertices(&walking));
    assert!(still.iter().zip(&moving).any(|(a, b)| a.distance(*b) > 0.1));
}

#[test]
fn light_levels_ride_along_the_vertices() {
    let pig = MobInstance {
        sky_level: 7.0,
        block_level: 12.0,
        ..instance()
    };
    assert!(vertices(&pig).iter().all(|v| v[6] == 7.0 && v[7] == 12.0));
}

#[test]
fn culled_mobs_emit_nothing() {
    assert!(visible_vertices(&[instance()], |_, _| false).is_empty());
    assert!(!visible_vertices(&[instance()], |_, _| true).is_empty());
}

#[test]
fn blob_vertices_carry_local_coordinates_and_opacity() {
    let pig = instance();
    let mut out = Vec::new();
    push_blobs(&pig, &mut out);
    assert_eq!(out.len(), 6 * BLOB_FLOATS);
    for v in out.as_chunks::<BLOB_FLOATS>().0 {
        assert_eq!(v[1], 64.02);
        assert_eq!(v[5], 0.5);
        let local = Vec2::new(v[3], v[4]);
        let expected = (Vec2::new(v[0], v[2]) - Vec2::new(10.0, -5.0)) / 0.7;
        assert!((local - expected).length() < 1e-5);
    }
}

#[test]
fn culled_mobs_emit_no_blobs() {
    assert!(visible_blobs(&[instance()], |_, _| false).is_empty());
    assert!(!visible_blobs(&[instance()], |_, _| true).is_empty());
}
