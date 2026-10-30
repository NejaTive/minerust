use super::*;

fn f32_at(bytes: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

#[test]
fn layout_matches_shader() {
    let globals = Globals {
        view_proj: Mat4::from_cols_array(&std::array::from_fn(|i| i as f32)),
        camera_pos: Vec3::new(100.0, 101.0, 102.0),
        fog_start: 103.0,
        fog_color: Vec3::new(104.0, 105.0, 106.0),
        fog_end: 107.0,
        eye_fluid: 108.0,
        darkness: 109.0,
        anim_time: 110.0,
        view_dir: Mat4::from_cols_array(&std::array::from_fn(|i| 200.0 + i as f32)),
        sun_dir: Vec3::new(300.0, 301.0, 302.0),
        night: 303.0,
        zenith: Vec3::new(304.0, 305.0, 306.0),
        warm: 307.0,
        horizon_tilt: 308.0,
        star_rotation: Vec2::new(309.0, 310.0),
    };
    let bytes = globals.to_bytes();
    assert_eq!(bytes.len(), 224);
    assert_eq!(f32_at(&bytes, 0), 0.0);
    assert_eq!(f32_at(&bytes, 60), 15.0);
    assert_eq!(f32_at(&bytes, 64), 100.0);
    assert_eq!(f32_at(&bytes, 72), 102.0);
    assert_eq!(f32_at(&bytes, 76), 103.0);
    assert_eq!(f32_at(&bytes, 80), 104.0);
    assert_eq!(f32_at(&bytes, 88), 106.0);
    assert_eq!(f32_at(&bytes, 92), 107.0);
    assert_eq!(f32_at(&bytes, 96), 108.0);
    assert_eq!(f32_at(&bytes, 100), 109.0);
    assert_eq!(f32_at(&bytes, 104), 110.0);
    assert!(bytes[108..112].iter().all(|&b| b == 0));
    assert_eq!(f32_at(&bytes, 112), 200.0);
    assert_eq!(f32_at(&bytes, 172), 215.0);
    assert_eq!(f32_at(&bytes, 176), 300.0);
    assert_eq!(f32_at(&bytes, 184), 302.0);
    assert_eq!(f32_at(&bytes, 188), 303.0);
    assert_eq!(f32_at(&bytes, 192), 304.0);
    assert_eq!(f32_at(&bytes, 200), 306.0);
    assert_eq!(f32_at(&bytes, 204), 307.0);
    assert_eq!(f32_at(&bytes, 208), 308.0);
    assert_eq!(f32_at(&bytes, 212), 309.0);
    assert_eq!(f32_at(&bytes, 216), 310.0);
    assert!(bytes[220..].iter().all(|&b| b == 0));
}

#[test]
fn view_dir_recovers_pixel_direction() {
    let eye = Vec3::new(5000.0, 70.0, -3000.0);
    let forward = Vec3::new(0.3, 0.4, -0.8).normalize();
    let view = glam::camera::rh::view::look_to_mat4(eye, forward, Vec3::Y);
    let proj = glam::camera::rh::proj::directx::perspective(1.2, 1.5, 0.1, 300.0);
    let inverse = view_dir_matrix(proj * view);
    for (x, y) in [(0.0, 0.0), (0.7, -0.4), (-1.0, 1.0)] {
        let dir = (inverse * glam::Vec4::new(x, y, 1.0, 0.0))
            .truncate()
            .normalize();
        let clip = proj * view * (eye + dir * 10.0).extend(1.0);
        assert!((clip.x / clip.w - x).abs() < 1e-3, "{x} {y}");
        assert!((clip.y / clip.w - y).abs() < 1e-3, "{x} {y}");
        assert!(clip.w > 0.0);
    }
    let center = (inverse * glam::Vec4::new(0.0, 0.0, 1.0, 0.0))
        .truncate()
        .normalize();
    assert!((center - forward).length() < 1e-4);
}
