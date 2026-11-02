use super::*;

fn camera() -> Mat4 {
    let view = glam::camera::rh::view::look_to_mat4(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
    let proj = glam::camera::rh::proj::directx::perspective(1.2, 1.5, 0.1, 300.0);
    proj * view
}

#[test]
fn sun_ahead_is_centered_and_visible() {
    let sun = Vec3::new(0.0, 0.5, -1.0).normalize();
    let (uv, strength) = light_rays(camera(), Vec3::ZERO, sun, 0.0, 0.0, false);
    assert!(uv.x > 0.4 && uv.x < 0.6, "{uv}");
    assert!(strength > 0.99);
}

#[test]
fn sun_behind_the_camera_gives_no_rays() {
    let sun = Vec3::new(0.0, 0.5, 1.0).normalize();
    assert_eq!(
        light_rays(camera(), Vec3::ZERO, sun, 0.0, 0.0, false).1,
        0.0
    );
}

#[test]
fn rays_fade_in_rain_and_underwater() {
    let sun = Vec3::new(0.0, 0.5, -1.0).normalize();
    assert_eq!(
        light_rays(camera(), Vec3::ZERO, sun, 0.0, 1.0, false).1,
        0.0
    );
    assert_eq!(light_rays(camera(), Vec3::ZERO, sun, 0.0, 0.0, true).1, 0.0);
    let moon_sun = Vec3::new(0.0, -0.5, 1.0).normalize();
    assert_eq!(
        light_rays(camera(), Vec3::ZERO, moon_sun, 1.0, 1.0, false).1,
        0.0
    );
    assert_eq!(
        light_rays(camera(), Vec3::ZERO, moon_sun, 1.0, 0.0, true).1,
        0.0
    );
}

#[test]
fn sun_gives_no_rays_at_full_night() {
    let sun = Vec3::new(0.0, 0.5, -1.0).normalize();
    assert_eq!(
        light_rays(camera(), Vec3::ZERO, sun, 1.0, 0.0, false).1,
        0.0
    );
}

#[test]
fn moon_ahead_gives_weaker_rays_than_the_sun() {
    let sun_dir = Vec3::new(0.0, 0.5, -1.0).normalize();
    let (_, day) = light_rays(camera(), Vec3::ZERO, sun_dir, 0.0, 0.0, false);
    let moon_sun = Vec3::new(0.0, -0.5, 1.0).normalize();
    let (uv, night) = light_rays(camera(), Vec3::ZERO, moon_sun, 1.0, 0.0, false);
    assert!(uv.x > 0.4 && uv.x < 0.6, "{uv}");
    assert!(night > 0.0 && night < day, "{night} {day}");
    assert!((0.25..=0.35).contains(&(night / day)));
}

#[test]
fn moon_behind_the_camera_gives_no_rays() {
    let moon_sun = Vec3::new(0.0, -0.5, -1.0).normalize();
    assert_eq!(
        light_rays(camera(), Vec3::ZERO, moon_sun, 1.0, 0.0, false).1,
        0.0
    );
}

#[test]
fn rays_vanish_at_the_horizon_on_both_sides() {
    for y in [0.0, 1e-4, -1e-4] {
        let dir = Vec3::new(0.0, y, -1.0).normalize();
        for night in [0.0, 1.0] {
            assert!(light_rays(camera(), Vec3::ZERO, dir, night, 0.0, false).1 < 1e-3);
        }
    }
}
