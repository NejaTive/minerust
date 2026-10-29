use super::*;

fn camera() -> Camera {
    Camera::spawn(Vec3::ZERO)
}

#[test]
fn defaults() {
    let c = camera();
    assert_eq!(c.sensitivity(), 1.0);
    assert_eq!(c.fov(), 70.0);
}

#[test]
fn sensitivity_is_clamped() {
    let mut c = camera();
    c.set_sensitivity(10.0);
    assert_eq!(c.sensitivity(), MAX_SENSITIVITY);
    c.set_sensitivity(0.0);
    assert_eq!(c.sensitivity(), MIN_SENSITIVITY);
    c.set_sensitivity(1.5);
    assert_eq!(c.sensitivity(), 1.5);
    c.set_sensitivity(f32::NAN);
    assert_eq!(c.sensitivity(), 1.5);
}

#[test]
fn fov_is_clamped() {
    let mut c = camera();
    c.set_fov(200.0);
    assert_eq!(c.fov(), MAX_FOV);
    c.set_fov(-5.0);
    assert_eq!(c.fov(), MIN_FOV);
    c.set_fov(90.0);
    assert_eq!(c.fov(), 90.0);
    c.set_fov(f32::NAN);
    assert_eq!(c.fov(), 90.0);
}

#[test]
fn sensitivity_scales_rotation() {
    let mut slow = camera();
    slow.yaw = 0.0;
    slow.set_sensitivity(0.5);
    slow.rotate(100.0, 0.0);
    let mut fast = camera();
    fast.yaw = 0.0;
    fast.set_sensitivity(2.0);
    fast.rotate(100.0, 0.0);
    assert!((fast.yaw / slow.yaw - 4.0).abs() < 1e-5);
}

#[test]
fn fov_changes_projection() {
    let mut c = camera();
    // Точка сбоку от взгляда: матрица включает поворот, так что по `x_axis.x` угол
    // обзора не сравнить. Тест раньше шёл только на wasm32 и этого не замечал.
    let right = c.forward().cross(Vec3::Y).normalize();
    let point = (c.position + c.forward() * 10.0 + right * 3.0).extend(1.0);
    let ndc_x = |c: &Camera| {
        let clip = c.view_proj(1.0, 100.0, 1.0) * point;
        (clip.x / clip.w).abs()
    };
    c.set_fov(60.0);
    let narrow = ndc_x(&c);
    c.set_fov(110.0);
    let wide = ndc_x(&c);
    // Больше угол — меньше масштаб по X/Y.
    assert!(wide < narrow);
}
