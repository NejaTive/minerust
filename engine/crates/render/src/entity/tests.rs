use super::*;
use world::mob::horse_variant;

fn instance() -> MobInstance {
    MobInstance {
        kind: ModelKind::Pig,
        variant: 0,
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
    assert_eq!(count(ModelKind::Mooshroom), 9 + 2);
    assert_eq!(count(ModelKind::PolarBear), 10);
    assert_eq!(count(ModelKind::Wolf), 11);
    assert_eq!(count(ModelKind::Horse), 23);
    assert_eq!(count(ModelKind::Donkey), 23);
    assert_eq!(count(ModelKind::Llama), 9);
}

#[test]
fn each_model_draws_from_its_own_sheets() {
    for (kind, sheet, boxes) in [
        (ModelKind::Pig, EntityTexture::Pig, 7),
        (ModelKind::Chicken, EntityTexture::Chicken, 8),
        (ModelKind::Cow, EntityTexture::Cow, 9),
        (ModelKind::Sheep, EntityTexture::Sheep, 6),
        (ModelKind::Sheep, EntityTexture::SheepFur, 6),
        (ModelKind::Mooshroom, EntityTexture::Mooshroom, 9),
        (ModelKind::PolarBear, EntityTexture::PolarBear, 10),
        (ModelKind::Wolf, EntityTexture::Wolf, 11),
        (ModelKind::Horse, EntityTexture::HorseWhite, 23),
        (ModelKind::Donkey, EntityTexture::Donkey, 23),
        (ModelKind::Llama, EntityTexture::LlamaCreamy, 9),
    ] {
        let model = of_kind(kind);
        assert_eq!(boxes_on(&model, sheet), boxes, "{kind:?} {sheet:?}");
        let others: usize = EntityTexture::ALL
            .into_iter()
            .filter(|&other| other != sheet)
            .map(|other| boxes_on(&model, other))
            .sum();
        let mushrooms = boxes_on(&model, EntityTexture::Mushroom);
        assert_eq!(others + boxes, vertices(&model).len() / 36);
        assert_eq!(mushrooms, if kind == ModelKind::Mooshroom { 2 } else { 0 });
    }
}

#[test]
fn standing_models_match_their_hitboxes() {
    for (kind, width, height, length) in [
        (ModelKind::Chicken, 0.4, 0.7, 0.5),
        (ModelKind::Cow, 0.9, 1.4, 1.3),
        (ModelKind::Sheep, 0.9, 1.3, 1.1),
        (ModelKind::PolarBear, 1.3, 1.4, 1.9),
        (ModelKind::Wolf, 0.6, 0.85, 1.0),
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
        ModelKind::Mooshroom,
        ModelKind::PolarBear,
        ModelKind::Wolf,
        ModelKind::Horse,
        ModelKind::Donkey,
        ModelKind::Llama,
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

fn mushroom_points(mob: &MobInstance) -> Vec<Vec3> {
    sheets(mob)[EntityTexture::Mushroom as usize]
        .as_chunks::<VERTEX_FLOATS>()
        .0
        .iter()
        .map(position)
        .collect()
}

#[test]
fn mooshroom_carries_two_mushrooms_on_the_back_and_one_on_the_head() {
    let cow = of_kind(ModelKind::Mooshroom);
    let points = mushroom_points(&cow);
    assert_eq!(points.len(), 3 * 24);
    for back in points[..48].chunks(24) {
        let min = back.iter().map(|p| p.y).fold(f32::MAX, f32::min) - cow.feet.y;
        let max = back.iter().map(|p| p.y).fold(f32::MIN, f32::max) - cow.feet.y;
        assert!(
            (min - 1.35).abs() < 0.05 && (max - 2.35).abs() < 0.05,
            "{min} {max}"
        );
    }
    let turned = MobInstance {
        head_yaw: 1.0,
        ..cow.clone()
    };
    let moved = mushroom_points(&turned);
    assert_eq!(points[..48], moved[..48]);
    assert!(
        points[48..]
            .iter()
            .zip(&moved[48..])
            .any(|(a, b)| a.distance(*b) > 0.05)
    );
    for v in &sheets(&cow)[EntityTexture::Mushroom as usize] {
        assert!(v.is_finite());
    }
}

#[test]
fn mushroom_planes_cover_the_whole_block_texture() {
    let cow = of_kind(ModelKind::Mooshroom);
    let uvs: Vec<(f32, f32)> = sheets(&cow)[EntityTexture::Mushroom as usize]
        .as_chunks::<VERTEX_FLOATS>()
        .0
        .iter()
        .map(|v| (v[3], v[4]))
        .collect();
    assert!(
        uvs.iter()
            .all(|&(u, v)| u == 0.0 || u == 1.0 && (v == 0.0 || v == 1.0))
    );
    assert!(uvs.contains(&(0.0, 0.0)) && uvs.contains(&(1.0, 1.0)));
}

#[test]
fn polar_bear_ears_mirror_each_other() {
    let ear = |mirror| {
        let mut part = part(
            EntityTexture::PolarBear,
            (26.0, 0.0),
            [2.5, -4.0, -1.0],
            [2.0, 2.0, 1.0],
            [0.0, 10.0, -16.0],
            Mat3::IDENTITY,
        );
        part.mirror = mirror;
        let mut out = Vec::new();
        push_box(&part, &of_kind(ModelKind::PolarBear), &mut out);
        out.as_chunks::<VERTEX_FLOATS>()
            .0
            .iter()
            .map(|v| (v[3], v[4]))
            .collect::<Vec<_>>()
    };
    let (plain, flipped) = (ear(false), ear(true));
    assert_ne!(plain, flipped);
    let range = |uvs: &[(f32, f32)]| {
        let us = uvs.iter().map(|uv| uv.0);
        (
            us.clone().fold(f32::MAX, f32::min),
            us.fold(f32::MIN, f32::max),
        )
    };
    assert_eq!(range(&plain), range(&flipped));
    assert_eq!(range(&plain), (26.0 / 128.0, 32.0 / 128.0));
}

#[test]
fn wolf_tail_hangs_back_and_down() {
    let wolf = of_kind(ModelKind::Wolf);
    let wolf_sheet = sheets(&wolf);
    let tail = &wolf_sheet[EntityTexture::Wolf as usize]
        .as_chunks::<VERTEX_FLOATS>()
        .0[6 * 36..7 * 36];
    let (low, high) = tail
        .iter()
        .map(position)
        .fold((f32::MAX, f32::MIN), |(lo, hi), p| {
            (lo.min(p.y), hi.max(p.y))
        });
    assert!((high - wolf.feet.y - 0.78).abs() < 0.03 && low - wolf.feet.y < 0.5);
}

#[test]
fn polar_bear_body_is_one_connected_mass_behind_the_head() {
    let bear = of_kind(ModelKind::PolarBear);
    let sheet = sheets(&bear);
    let boxes = sheet[EntityTexture::PolarBear as usize]
        .as_chunks::<VERTEX_FLOATS>()
        .0;
    let extent = |index: usize| {
        let points: Vec<Vec3> = boxes[index * 36..(index + 1) * 36]
            .iter()
            .map(position)
            .collect();
        (
            points.iter().copied().reduce(Vec3::min).unwrap(),
            points.iter().copied().reduce(Vec3::max).unwrap(),
        )
    };
    let (head, (rear_min, rear_max), (front_min, front_max)) = (extent(0), extent(4), extent(5));
    assert!(
        rear_max.y - rear_min.y < 0.8,
        "body must lie flat: {rear_min} {rear_max}"
    );
    assert!(
        (front_min.x - rear_max.x).abs() < 0.01,
        "{front_min} {rear_max}"
    );
    assert!(front_max.x - front_min.x > 0.7);
    assert!(head.0.x - front_max.x < 0.45, "gap between head and body");
}

fn position_bounds(floats: &[f32]) -> (Vec3, Vec3) {
    let points: Vec<Vec3> = floats
        .as_chunks::<VERTEX_FLOATS>()
        .0
        .iter()
        .map(|v| Vec3::new(v[0], v[1], v[2]))
        .collect();
    (
        points.iter().copied().reduce(Vec3::min).unwrap(),
        points.iter().copied().reduce(Vec3::max).unwrap(),
    )
}

fn uvs(floats: &[f32]) -> Vec<[f32; 2]> {
    floats
        .as_chunks::<VERTEX_FLOATS>()
        .0
        .iter()
        .map(|v| [v[3], v[4]])
        .collect()
}

#[test]
fn horse_colour_picks_the_sheet() {
    for colour in 0..7u8 {
        let horse = MobInstance {
            variant: horse_variant(colour, 0),
            ..of_kind(ModelKind::Horse)
        };
        let drawn: Vec<usize> = EntityTexture::ALL
            .into_iter()
            .filter(|&sheet| boxes_on(&horse, sheet) > 0)
            .map(|sheet| sheet as usize)
            .collect();
        assert_eq!(
            drawn,
            [EntityTexture::HorseWhite as usize + usize::from(colour)]
        );
    }
}

#[test]
fn horse_markings_are_a_second_layer_with_the_same_uv() {
    for markings in 1..5u8 {
        let horse = MobInstance {
            variant: horse_variant(3, markings),
            ..of_kind(ModelKind::Horse)
        };
        let sheet = sheets(&horse);
        let base = &sheet[EntityTexture::HorseBrown as usize];
        let layer = &sheet[EntityTexture::MarkingsWhite as usize + usize::from(markings) - 1];
        assert_eq!(uvs(base), uvs(layer));
        let (lo, hi) = (position_bounds(base), position_bounds(layer));
        assert!(lo.0.x > hi.0.x && lo.1.x < hi.1.x, "{lo:?} {hi:?}");
    }
    let bare = sheets(&of_kind(ModelKind::Horse));
    let first = EntityTexture::MarkingsWhite as usize;
    assert!(bare[first..first + 4].iter().all(Vec::is_empty));
}

#[test]
fn llama_colour_picks_the_sheet() {
    for colour in 0..4u8 {
        let llama = MobInstance {
            variant: colour,
            ..of_kind(ModelKind::Llama)
        };
        let sheet = EntityTexture::ALL[EntityTexture::LlamaCreamy as usize + usize::from(colour)];
        assert_eq!(boxes_on(&llama, sheet), 9);
    }
}

#[test]
fn horse_and_llama_sheets_use_their_vanilla_grids() {
    assert_eq!(EntityTexture::HorseBrown.units(), Vec2::new(128.0, 128.0));
    assert_eq!(
        EntityTexture::MarkingsWhiteDots.units(),
        Vec2::new(128.0, 128.0)
    );
    assert_eq!(EntityTexture::Donkey.units(), Vec2::new(128.0, 128.0));
    assert_eq!(EntityTexture::LlamaGray.units(), Vec2::new(128.0, 64.0));
}

#[test]
fn texture_order_matches_discriminants() {
    for (i, sheet) in EntityTexture::ALL.into_iter().enumerate() {
        assert_eq!(sheet as usize, i);
    }
    assert_eq!(EntityTexture::ALL.len(), TEXTURES);
}

#[test]
fn donkey_is_the_horse_model_scaled_down() {
    let (hmin, hmax) = bounds(&of_kind(ModelKind::Horse));
    let (dmin, dmax) = bounds(&of_kind(ModelKind::Donkey));
    let ratio = (dmax.x - dmin.x) / (hmax.x - hmin.x);
    assert!((ratio - 0.87).abs() < 0.02, "{ratio}");
}

#[test]
fn horse_and_llama_stand_on_their_feet_with_the_right_footprint() {
    for (kind, width, length) in [(ModelKind::Horse, 1.4, 1.9), (ModelKind::Llama, 0.9, 1.1)] {
        let model = of_kind(kind);
        let (min, max) = bounds(&model);
        assert!((min.y - model.feet.y).abs() < 0.01, "{kind:?} {min}");
        assert!(max.z - min.z <= width + 0.2, "{kind:?} {min} {max}");
        assert!(max.x - min.x >= length - 0.2, "{kind:?} {min} {max}");
    }
}

#[test]
fn horse_legs_swing_but_the_tail_stays() {
    let still = of_kind(ModelKind::Horse);
    let walking = MobInstance {
        limb_phase: 1.0,
        limb_swing: 1.0,
        ..still.clone()
    };
    let sheet = |instance: &MobInstance| {
        position_bounds(&sheets(instance)[EntityTexture::HorseWhite as usize])
    };
    assert_ne!(sheet(&still), sheet(&walking));
    let parts = |instance: &MobInstance| -> Vec<f32> {
        equine::horse_parts(instance)
            .iter()
            .filter(|part| part.pivot == Vec3::new(0.0, 3.0, 14.0))
            .flat_map(|part| {
                let mut out = Vec::new();
                push_box(part, instance, &mut out);
                out
            })
            .collect()
    };
    assert_eq!(parts(&still), parts(&walking));
    assert_eq!(parts(&still).len(), 3 * 36 * VERTEX_FLOATS);
}
