use glam::camera::rh::{proj::directx, view};

use super::*;

const NEAR: f32 = 0.1;
const FAR: f32 = 160.0;
const TOP: f32 = 128.0;

fn camera(eye: Vec3, forward: Vec3) -> Mat4 {
    let proj = directx::perspective(1.2, 16.0 / 9.0, NEAR, FAR);
    proj * view::look_to_mat4(eye, forward, Vec3::Y)
}

fn light() -> Vec3 {
    Vec3::new(0.4, 0.8, 0.3).normalize()
}

fn texel_coords(cascades: &Cascades, i: usize, p: Vec3, resolution: u32) -> glam::Vec2 {
    let clip = cascades.matrices[i] * p.extend(1.0);
    glam::Vec2::new(clip.x, clip.y) * (resolution as f32 / 2.0)
}

#[test]
fn light_dir_changes_only_past_threshold() {
    let base = light();
    let nudge = |degrees: f32| {
        let angle = degrees.to_radians();
        (base * angle.cos() + base.cross(Vec3::Y).normalize() * angle.sin()).normalize()
    };
    assert_eq!(quantize_light_dir(None, base), base);
    assert_eq!(quantize_light_dir(Some(base), nudge(0.03)), base);
    assert_eq!(quantize_light_dir(Some(base), nudge(-0.05)), base);
    let moved = nudge(0.3);
    assert_eq!(quantize_light_dir(Some(base), moved), moved);
}

#[test]
fn matrices_are_identical_for_sub_threshold_sun_motion() {
    let vp = camera(Vec3::new(10.0, 70.0, -5.0), Vec3::NEG_Z);
    let base = light();
    let nudged = (base + Vec3::new(0.0, 0.0, 2e-4)).normalize();
    let a = compute_cascades(vp, NEAR, FAR, base, ShadowQuality::High, TOP);
    let dir = quantize_light_dir(Some(base), nudged);
    let b = compute_cascades(vp, NEAR, FAR, dir, ShadowQuality::High, TOP);
    assert_eq!(a.matrices, b.matrices);
}

#[test]
fn splits_grow_and_end_at_far() {
    let splits = split_distances(NEAR, FAR, 3);
    assert!(NEAR < splits[0] && splits[0] < splits[1] && splits[1] < splits[2]);
    assert_eq!(splits[2], FAR);
    assert_eq!(split_distances(NEAR, FAR, 2)[1], FAR);
}

#[test]
fn cascades_cover_camera_frustum() {
    let vp = camera(
        Vec3::new(10.0, 70.0, -5.0),
        Vec3::new(0.3, -0.2, -0.9).normalize(),
    );
    let inverse = vp.inverse();
    for quality in [ShadowQuality::Low, ShadowQuality::High] {
        let cascades = compute_cascades(vp, NEAR, FAR, light(), quality, TOP);
        let mut from = NEAR;
        for i in 0..cascades.count {
            let to = cascades.splits[i];
            for corner in slice_corners(inverse, NEAR, FAR, from, to) {
                let clip = cascades.matrices[i] * corner.extend(1.0);
                assert!(clip.x.abs() <= 1.0 + 1e-3, "x {clip:?} cascade {i}");
                assert!(clip.y.abs() <= 1.0 + 1e-3, "y {clip:?} cascade {i}");
                assert!((0.0..=1.0).contains(&clip.z), "z {clip:?} cascade {i}");
            }
            from = to;
        }
    }
}

#[test]
fn casters_toward_sun_stay_in_depth_range() {
    let eye = Vec3::new(0.0, 70.0, 0.0);
    let vp = camera(eye, Vec3::NEG_Z);
    let cascades = compute_cascades(vp, NEAR, FAR, light(), ShadowQuality::High, TOP);
    let high = eye + light() * 40.0;
    let clip = cascades.matrices[0] * high.extend(1.0);
    assert!((0.0..=1.0).contains(&clip.z), "{clip:?}");
}

#[test]
fn texel_grid_is_stable_under_sub_texel_camera_shift() {
    let forward = Vec3::new(0.2, -0.1, -1.0).normalize();
    let resolution = ShadowQuality::High.resolution();
    let eye = Vec3::new(100.0, 70.0, 100.0);
    let base = compute_cascades(
        camera(eye, forward),
        NEAR,
        FAR,
        light(),
        ShadowQuality::High,
        TOP,
    );
    let point = Vec3::new(103.3, 64.0, 90.7);
    for i in 0..3 {
        let texel = base.texel_world[i];
        let shifted = camera(eye + Vec3::new(0.3, 0.1, -0.2) * texel, forward);
        let moved = compute_cascades(shifted, NEAR, FAR, light(), ShadowQuality::High, TOP);
        let a = texel_coords(&base, i, point, resolution);
        let b = texel_coords(&moved, i, point, resolution);
        let delta = b - a;
        assert!((delta.x - delta.x.round()).abs() < 1e-2, "{i} {delta}");
        assert!((delta.y - delta.y.round()).abs() < 1e-2, "{i} {delta}");
    }
}

#[test]
fn radius_does_not_depend_on_camera_rotation() {
    let eye = Vec3::new(0.0, 70.0, 0.0);
    let a = compute_cascades(
        camera(eye, Vec3::NEG_Z),
        NEAR,
        FAR,
        light(),
        ShadowQuality::High,
        TOP,
    );
    let b = compute_cascades(
        camera(eye, Vec3::new(1.0, 0.3, 0.2).normalize()),
        NEAR,
        FAR,
        light(),
        ShadowQuality::High,
        TOP,
    );
    assert_eq!(a.texel_world, b.texel_world);
}

#[test]
fn light_follows_sun_by_day_and_moon_by_night() {
    let day = shadow_light(Vec3::new(0.0, 0.8, 0.6), 0.0, 0.0).unwrap();
    assert_eq!(day.dir, Vec3::new(0.0, 0.8, 0.6));
    assert!(day.strength > 0.99);
    let night = shadow_light(Vec3::new(0.0, -0.8, -0.6), 0.0, 0.0).unwrap();
    assert_eq!(night.dir, Vec3::new(0.0, 0.8, 0.6));
    assert!(night.strength < 0.35);
    assert!(shadow_light(Vec3::new(1.0, 0.01, 0.0), 0.0, 0.0).is_none());
    let rainy = shadow_light(Vec3::Y, 0.0, 1.0).unwrap();
    assert!(rainy.strength < day.strength);
}

#[test]
fn layout_matches_shader() {
    let mut matrices = [Mat4::ZERO; MAX_CASCADES];
    matrices[0] = Mat4::from_cols_array(&std::array::from_fn(|i| i as f32));
    matrices[2] = Mat4::from_cols_array(&std::array::from_fn(|i| 100.0 + i as f32));
    let data = ShadowData {
        cascades: Cascades {
            count: 2,
            matrices,
            splits: [10.0, 20.0, 30.0],
            texel_world: [0.5, 0.6, 0.7],
        },
        light: ShadowLight {
            dir: Vec3::new(1.0, 2.0, 3.0),
            strength: 0.25,
        },
        quality: ShadowQuality::Low,
        cloud_strength: 0.4,
    };
    let bytes = data.to_bytes();
    let f = |offset: usize| f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
    assert_eq!(f(0), 0.0);
    assert_eq!(f(60), 15.0);
    assert_eq!(f(128), 100.0);
    assert_eq!(f(188), 115.0);
    assert_eq!([f(192), f(196), f(200)], [10.0, 20.0, 30.0]);
    assert_eq!([f(208), f(212), f(216), f(220)], [1.0, 2.0, 3.0, 0.25]);
    assert_eq!([f(224), f(228), f(232)], [0.5, 0.6, 0.7]);
    assert_eq!(f(240), 2.0);
    assert_eq!(f(244), 2.0);
    assert_eq!(f(248), 1.0 / 1024.0);
    assert_eq!(f(252), 0.4);
}
