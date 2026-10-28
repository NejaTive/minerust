use glam::camera::rh::{proj::directx, view};
use glam::{Mat4, Vec3};

/// Вертикальный угол обзора по умолчанию, градусы.
const DEFAULT_FOV: f32 = 70.0;
/// Пределы угла обзора, градусы: ниже 60 картинка «туннельная», выше 110 края сильно растянуты.
pub const MIN_FOV: f32 = 60.0;
pub const MAX_FOV: f32 = 110.0;
const NEAR: f32 = 0.1;
/// Поворот камеры при множителе 1, радиан на пиксель движения мыши.
const BASE_SENSITIVITY: f32 = 0.002;
/// Пределы множителя чувствительности; дублируются в `src/settings.ts`.
pub const MIN_SENSITIVITY: f32 = 0.25;
pub const MAX_SENSITIVITY: f32 = 3.0;
const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;
/// Наклон на спауне: −15°, горизонт в верхней части экрана.
const SPAWN_PITCH: f32 = -15.0 * std::f32::consts::PI / 180.0;

/// Камера от первого лица: yaw/pitch в радианах, Y — вверх. Позицию задаёт игрок.
pub struct Camera {
    pub position: Vec3,
    /// Поворот вокруг Y; при yaw = 0 камера смотрит вдоль +X, при -PI/2 — вдоль -Z.
    pub yaw: f32,
    /// Наклон вверх/вниз, ограничен ±89°.
    pub pitch: f32,
    /// Множитель к [`BASE_SENSITIVITY`], в [`MIN_SENSITIVITY`]..=[`MAX_SENSITIVITY`].
    sensitivity: f32,
    /// Вертикальный угол обзора в градусах, в [`MIN_FOV`]..=[`MAX_FOV`].
    fov: f32,
}

impl Camera {
    /// Камера на спауне `position`: смотрит вдоль +X и чуть вниз, на рельеф впереди.
    pub fn spawn(position: Vec3) -> Self {
        Self {
            position,
            yaw: 0.0,
            pitch: SPAWN_PITCH,
            sensitivity: 1.0,
            fov: DEFAULT_FOV,
        }
    }

    /// Камера из `[x, y, z, yaw, pitch]` (начало `Engine::camera_state`); `None`, если
    /// длина не 5 или есть не-числа.
    pub fn from_state(state: &[f32]) -> Option<Self> {
        let &[x, y, z, yaw, pitch] = state else {
            return None;
        };
        if !state.iter().all(|v| v.is_finite()) {
            return None;
        }
        Some(Self {
            position: Vec3::new(x, y, z),
            yaw,
            pitch: pitch.clamp(-MAX_PITCH, MAX_PITCH),
            sensitivity: 1.0,
            fov: DEFAULT_FOV,
        })
    }

    /// Направление взгляда с учётом pitch.
    pub fn forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(cy * cp, sp, sy * cp)
    }

    /// Множитель чувствительности мыши; значение вне пределов зажимается, NaN игнорируется.
    pub fn set_sensitivity(&mut self, factor: f32) {
        if !factor.is_nan() {
            self.sensitivity = factor.clamp(MIN_SENSITIVITY, MAX_SENSITIVITY);
        }
    }

    pub fn sensitivity(&self) -> f32 {
        self.sensitivity
    }

    /// Вертикальный угол обзора в градусах; значение вне пределов зажимается, NaN игнорируется.
    pub fn set_fov(&mut self, degrees: f32) {
        if !degrees.is_nan() {
            self.fov = degrees.clamp(MIN_FOV, MAX_FOV);
        }
    }

    pub fn fov(&self) -> f32 {
        self.fov
    }

    /// Мышь вправо — поворот вправо, вниз — взгляд вниз.
    pub fn rotate(&mut self, dx: f32, dy: f32) {
        let step = BASE_SENSITIVITY * self.sensitivity;
        self.yaw = (self.yaw + dx * step) % std::f32::consts::TAU;
        self.pitch = (self.pitch - dy * step).clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Матрица view-projection; `far` — дальняя плоскость в блоках, см. `Fog::far_plane`.
    /// `fov_scale` множит угол обзора на кадр (бег), не меняя настройку `fov`.
    /// В glam 0.34 бывший `Mat4::perspective_rh` — это `rh::proj::directx`:
    /// NDC с глубиной 0..1 и Y вверх, как ожидает wgpu.
    pub fn view_proj(&self, aspect: f32, far: f32, fov_scale: f32) -> Mat4 {
        let fov = (self.fov * fov_scale).to_radians();
        let proj = directx::perspective(fov, aspect, NEAR, far);
        let view = view::look_to_mat4(self.position, self.forward(), Vec3::Y);
        proj * view
    }
}

#[cfg(test)]
mod tests {
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
}
