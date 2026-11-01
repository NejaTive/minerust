use super::*;

fn camera() -> Mat4 {
    let view = glam::camera::rh::view::look_to_mat4(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
    let proj = glam::camera::rh::proj::directx::perspective(1.2, 1.5, 0.1, 300.0);
    proj * view
}

#[test]
fn sun_ahead_is_centered_and_visible() {
    let sun = Vec3::new(0.0, 0.5, -1.0).normalize();
    let (uv, strength) = sun_rays(camera(), Vec3::ZERO, sun, 0.0, 0.0, false);
    assert!(uv.x > 0.4 && uv.x < 0.6, "{uv}");
    assert!(strength > 0.99);
}

#[test]
fn sun_behind_the_camera_gives_no_rays() {
    let sun = Vec3::new(0.0, 0.5, 1.0).normalize();
    assert_eq!(sun_rays(camera(), Vec3::ZERO, sun, 0.0, 0.0, false).1, 0.0);
}

#[test]
fn rays_fade_at_night_in_rain_and_underwater() {
    let sun = Vec3::new(0.0, 0.5, -1.0).normalize();
    assert_eq!(sun_rays(camera(), Vec3::ZERO, sun, 1.0, 0.0, false).1, 0.0);
    assert_eq!(sun_rays(camera(), Vec3::ZERO, sun, 0.0, 1.0, false).1, 0.0);
    assert_eq!(sun_rays(camera(), Vec3::ZERO, sun, 0.0, 0.0, true).1, 0.0);
}
