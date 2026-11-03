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
        flap_phase: 0.0,
        flap_amount: 0.0,
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

fn sheets(instance: &MobInstance) -> MobVertices {
    let mut out = MobVertices::default();
    push_mob(instance, &mut out);
    out
}

fn vertices(instance: &MobInstance) -> Vec<[f32; VERTEX_FLOATS]> {
    sheets(instance)
        .iter()
        .flat_map(|sheet| sheet.as_chunks::<VERTEX_FLOATS>().0.to_vec())
        .collect()
}

fn of_kind(kind: ModelKind) -> MobInstance {
    MobInstance { kind, ..instance() }
}

fn bounds(instance: &MobInstance) -> (Vec3, Vec3) {
    let points: Vec<Vec3> = vertices(instance).iter().map(position).collect();
    (
        points.iter().copied().reduce(Vec3::min).unwrap(),
        points.iter().copied().reduce(Vec3::max).unwrap(),
    )
}

fn boxes_on(instance: &MobInstance, sheet: EntityTexture) -> usize {
    sheets(instance)[sheet as usize].len() / VERTEX_FLOATS / 36
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
    assert!(
        visible_vertices(&[instance()], |_, _| false)
            .iter()
            .all(Vec::is_empty)
    );
    assert!(
        visible_vertices(&[instance()], |_, _| true)
            .iter()
            .any(|v| !v.is_empty())
    );
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

#[test]
fn models_have_the_minecraft_box_counts() {
    let count = |kind| vertices(&of_kind(kind)).len() / 36;
    assert_eq!(count(ModelKind::Pig), 7);
    assert_eq!(count(ModelKind::Chicken), 8);
    assert_eq!(count(ModelKind::Cow), 9);
    assert_eq!(count(ModelKind::Sheep), 12);
}

#[test]
fn each_model_draws_from_its_own_sheets() {
    for (kind, sheet, boxes) in [
        (ModelKind::Pig, EntityTexture::Pig, 7),
        (ModelKind::Chicken, EntityTexture::Chicken, 8),
        (ModelKind::Cow, EntityTexture::Cow, 9),
        (ModelKind::Sheep, EntityTexture::Sheep, 6),
        (ModelKind::Sheep, EntityTexture::SheepFur, 6),
    ] {
        let model = of_kind(kind);
        assert_eq!(boxes_on(&model, sheet), boxes, "{kind:?} {sheet:?}");
        let others: usize = EntityTexture::ALL
            .into_iter()
            .filter(|&other| other != sheet)
            .map(|other| boxes_on(&model, other))
            .sum();
        assert_eq!(others + boxes, vertices(&model).len() / 36);
    }
}

#[test]
fn standing_models_match_their_hitboxes() {
    for (kind, width, height, length) in [
        (ModelKind::Chicken, 0.4, 0.7, 0.5),
        (ModelKind::Cow, 0.9, 1.4, 1.3),
        (ModelKind::Sheep, 0.9, 1.3, 1.1),
    ] {
        let model = of_kind(kind);
        let (min, max) = bounds(&model);
        assert!((min.y - model.feet.y).abs() < 1e-4, "{kind:?} {min}");
        assert!(max.y - model.feet.y <= height + 0.3, "{kind:?} {max}");
        assert!(max.y - model.feet.y >= height - 0.3, "{kind:?} {max}");
        assert!(max.z - min.z <= width + 0.4, "{kind:?} {min} {max}");
        assert!(max.x - min.x >= length - 0.4, "{kind:?} {min} {max}");
    }
}

#[test]
fn every_model_stays_on_its_sheet_and_faces_outward() {
    for kind in [
        ModelKind::Pig,
        ModelKind::Chicken,
        ModelKind::Cow,
        ModelKind::Sheep,
    ] {
        let model = of_kind(kind);
        let verts = vertices(&model);
        for v in &verts {
            assert!(
                (0.0..=1.0).contains(&v[3]) && (0.0..=1.0).contains(&v[4]),
                "{kind:?} {v:?}"
            );
        }
        for tri in verts.as_chunks::<3>().0 {
            let [a, b, c] = [position(&tri[0]), position(&tri[1]), position(&tri[2])];
            assert!((b - a).cross(c - a).length() > 0.0, "{kind:?}");
        }
    }
}

#[test]
fn sheep_fur_is_larger_than_the_body_under_it() {
    let sheep = of_kind(ModelKind::Sheep);
    let extent = |sheet: EntityTexture| {
        let points: Vec<Vec3> = sheets(&sheep)[sheet as usize]
            .as_chunks::<VERTEX_FLOATS>()
            .0
            .iter()
            .map(position)
            .collect();
        let min = points.iter().copied().reduce(Vec3::min).unwrap();
        let max = points.iter().copied().reduce(Vec3::max).unwrap();
        max - min
    };
    let (fur, skin) = (
        extent(EntityTexture::SheepFur),
        extent(EntityTexture::Sheep),
    );
    assert!(fur.z > skin.z && fur.x > skin.x * 0.8);
}

#[test]
fn chicken_wings_spread_while_flapping() {
    let wings = |instance: &MobInstance| {
        sheets(instance)[EntityTexture::Chicken as usize]
            .as_chunks::<VERTEX_FLOATS>()
            .0[6 * 36..]
            .iter()
            .map(position)
            .collect::<Vec<_>>()
    };
    let rest = wings(&of_kind(ModelKind::Chicken));
    let flapping = wings(&MobInstance {
        flap_phase: 1.2,
        flap_amount: 1.0,
        ..of_kind(ModelKind::Chicken)
    });
    assert!(
        rest.iter()
            .zip(&flapping)
            .any(|(a, b)| a.distance(*b) > 0.1)
    );
    let width = |points: &[Vec3]| {
        let zs = points.iter().map(|p| p.z);
        zs.clone().fold(f32::MIN, f32::max) - zs.fold(f32::MAX, f32::min)
    };
    assert!(width(&flapping) > width(&rest));
}

#[test]
fn bottom_face_uv_matches_model_box() {
    let (u0, v0) = (10.0, 20.0);
    let (dx, dy, dz) = (4.0, 6.0, 2.0);
    let model = instance();
    let cube = part(
        EntityTexture::Pig,
        (u0, v0),
        [0.0, 0.0, 0.0],
        [dx, dy, dz],
        [0.0, 0.0, 0.0],
        Mat3::IDENTITY,
    );
    let mut out = Vec::new();
    push_box(&cube, &model, &mut out);
    let verts: Vec<[f32; VERTEX_FLOATS]> = out.as_chunks::<VERTEX_FLOATS>().0.to_vec();
    let uv_at = |corner: Vec3| {
        let at = to_world(corner, &model);
        let v = verts
            .iter()
            .find(|v| position(v).distance(at) < 1e-5 && (v[4] * 32.0 - v0).abs() <= dz + 1e-3)
            .expect("corner");
        Vec2::new(v[3] * 64.0, v[4] * 32.0)
    };
    let expect = |corner: Vec3, u: f32, v: f32| {
        let got = uv_at(corner);
        assert!((got - Vec2::new(u, v)).length() < 1e-3, "{corner}: {got}");
    };
    expect(Vec3::new(dx, dy, dz), u0 + dz + 2.0 * dx, v0);
    expect(Vec3::new(0.0, dy, dz), u0 + dz + dx, v0);
    expect(Vec3::new(0.0, dy, 0.0), u0 + dz + dx, v0 + dz);
    expect(Vec3::new(dx, dy, 0.0), u0 + dz + 2.0 * dx, v0 + dz);
}
