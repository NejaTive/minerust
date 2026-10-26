use glam::camera::rh::{proj::directx, view};
use glam::{Mat4, Vec3};

const FOV_Y: f32 = 70.0 * std::f32::consts::PI / 180.0;
const NEAR: f32 = 0.1;
/// Поворот камеры, радиан на пиксель движения мыши.
const SENSITIVITY: f32 = 0.002;
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
}

impl Camera {
    /// Камера на спауне `position`: смотрит вдоль +X и чуть вниз, на рельеф впереди.
    pub fn spawn(position: Vec3) -> Self {
        Self {
            position,
            yaw: 0.0,
            pitch: SPAWN_PITCH,
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
        })
    }

    /// Направление взгляда с учётом pitch.
    pub fn forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(cy * cp, sp, sy * cp)
    }

    /// Мышь вправо — поворот вправо, вниз — взгляд вниз.
    pub fn rotate(&mut self, dx: f32, dy: f32) {
        self.yaw = (self.yaw + dx * SENSITIVITY) % std::f32::consts::TAU;
        self.pitch = (self.pitch - dy * SENSITIVITY).clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Матрица view-projection; `far` — дальняя плоскость в блоках, см. `Fog::far_plane`.
    /// В glam 0.34 бывший `Mat4::perspective_rh` — это `rh::proj::directx`:
    /// NDC с глубиной 0..1 и Y вверх, как ожидает wgpu.
    pub fn view_proj(&self, aspect: f32, far: f32) -> Mat4 {
        let proj = directx::perspective(FOV_Y, aspect, NEAR, far);
        let view = view::look_to_mat4(self.position, self.forward(), Vec3::Y);
        proj * view
    }
}
