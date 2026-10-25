use std::collections::HashSet;

use glam::camera::rh::{proj::directx, view};
use glam::{Mat4, Vec3};

const FOV_Y: f32 = 70.0 * std::f32::consts::PI / 180.0;
const NEAR: f32 = 0.1;
const FAR: f32 = 1000.0;
/// Скорость полёта, блоков в секунду.
const SPEED: f32 = 8.0;
/// Поворот камеры, радиан на пиксель движения мыши.
const SENSITIVITY: f32 = 0.002;
const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;

/// Свободная камера как в креативе: yaw/pitch в радианах, Y — вверх.
pub struct Camera {
    pub position: Vec3,
    /// Поворот вокруг Y; при yaw = 0 камера смотрит вдоль +X, при -PI/2 — вдоль -Z.
    pub yaw: f32,
    /// Наклон вверх/вниз, ограничен ±89°.
    pub pitch: f32,
}

impl Camera {
    /// Стартует сзади-сверху от куба в начале координат и смотрит прямо на него.
    pub fn new() -> Self {
        let position = Vec3::new(0.0, 1.5, 4.0);
        let dir = -position;
        Self {
            position,
            yaw: dir.z.atan2(dir.x),
            pitch: dir.y.atan2(dir.x.hypot(dir.z)),
        }
    }

    /// Направление взгляда с учётом pitch.
    fn forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(cy * cp, sp, sy * cp)
    }

    /// Мышь вправо — поворот вправо, вниз — взгляд вниз.
    pub fn rotate(&mut self, dx: f32, dy: f32) {
        self.yaw = (self.yaw + dx * SENSITIVITY) % std::f32::consts::TAU;
        self.pitch = (self.pitch - dy * SENSITIVITY).clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Двигает камеру по нажатым клавишам (коды `KeyboardEvent.code`) за время `dt`.
    pub fn update(&mut self, keys: &HashSet<String>, dt: f32) {
        let pressed = |code: &str| keys.contains(code);
        // WASD — в горизонтальной плоскости, pitch не учитывается.
        let (sy, cy) = self.yaw.sin_cos();
        let forward = Vec3::new(cy, 0.0, sy);
        let right = Vec3::new(-sy, 0.0, cy);

        let mut dir = Vec3::ZERO;
        if pressed("KeyW") {
            dir += forward;
        }
        if pressed("KeyS") {
            dir -= forward;
        }
        if pressed("KeyD") {
            dir += right;
        }
        if pressed("KeyA") {
            dir -= right;
        }
        if pressed("Space") {
            dir += Vec3::Y;
        }
        if pressed("ShiftLeft") || pressed("ShiftRight") {
            dir -= Vec3::Y;
        }
        // Нормализуем, чтобы по диагонали не лететь быстрее.
        self.position += dir.normalize_or_zero() * SPEED * dt;
    }

    /// Матрица view-projection. В glam 0.34 бывший `Mat4::perspective_rh` — это
    /// `rh::proj::directx`: NDC с глубиной 0..1 и Y вверх, как ожидает wgpu.
    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let proj = directx::perspective(FOV_Y, aspect, NEAR, FAR);
        let view = view::look_to_mat4(self.position, self.forward(), Vec3::Y);
        proj * view
    }
}
