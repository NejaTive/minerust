use glam::camera::rh::{proj::directx, view};
use glam::{Mat4, Vec3};

/// Вертикальный угол обзора по умолчанию, градусы.
pub const DEFAULT_FOV: f32 = 70.0;
/// Пределы угла обзора, градусы: ниже 60 картинка «туннельная», выше 110 края сильно растянуты.
pub const MIN_FOV: f32 = 60.0;
pub const MAX_FOV: f32 = 110.0;
const NEAR: f32 = 0.1;
/// Поворот камеры при множителе 1, радиан на пиксель движения мыши.
const BASE_SENSITIVITY: f32 = 0.002;
/// Множитель чувствительности по умолчанию.
pub const DEFAULT_SENSITIVITY: f32 = 1.0;
/// Пределы множителя чувствительности.
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
            sensitivity: DEFAULT_SENSITIVITY,
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
            sensitivity: DEFAULT_SENSITIVITY,
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
mod tests;
