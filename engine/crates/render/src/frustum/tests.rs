use glam::camera::rh::{proj::directx, view};

use super::*;

const NEAR: f32 = 0.1;
const FAR: f32 = 100.0;

fn frustum() -> Frustum {
    let proj = directx::perspective(std::f32::consts::FRAC_PI_2, 1.0, NEAR, FAR);
    let view = view::look_to_mat4(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
    Frustum::from_view_proj(proj * view)
}

fn cube(center: Vec3, half: f32) -> (Vec3, Vec3) {
    (center - half, center + half)
}

fn visible((min, max): (Vec3, Vec3)) -> bool {
    frustum().intersects_aabb(min, max)
}

#[test]
fn box_in_front_is_visible() {
    assert!(visible(cube(Vec3::new(0.0, 0.0, -10.0), 1.0)));
}

#[test]
fn box_behind_is_hidden() {
    assert!(!visible(cube(Vec3::new(0.0, 0.0, 10.0), 1.0)));
}

#[test]
fn box_beside_view_is_hidden() {
    assert!(!visible(cube(Vec3::new(20.0, 0.0, -10.0), 1.0)));
    assert!(!visible(cube(Vec3::new(0.0, -20.0, -10.0), 1.0)));
}

#[test]
fn box_beyond_far_plane_is_hidden() {
    assert!(!visible(cube(Vec3::new(0.0, 0.0, -FAR - 2.0), 1.0)));
    assert!(visible(cube(Vec3::new(0.0, 0.0, -FAR + 0.5), 1.0)));
}

#[test]
fn box_just_before_near_plane_is_hidden() {
    let min = Vec3::new(-0.01, -0.01, -NEAR * 0.5);
    let max = Vec3::new(0.01, 0.01, -NEAR * 0.1);
    assert!(!visible((min, max)));
}

#[test]
fn box_around_camera_is_visible() {
    assert!(visible(cube(Vec3::ZERO, 5.0)));
}

#[test]
fn box_crossing_frustum_edge_is_visible() {
    assert!(visible(cube(Vec3::new(12.0, 0.0, -10.0), 3.0)));
    assert!(visible(cube(Vec3::new(1.0, 0.0, 0.0), 1.0)));
}
