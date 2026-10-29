use glam::camera::rh::{proj::directx, view};
use glam::{Mat4, Vec3};

pub const DEFAULT_FOV: f32 = 70.0;
pub const MIN_FOV: f32 = 60.0;
pub const MAX_FOV: f32 = 110.0;
const NEAR: f32 = 0.1;
const BASE_SENSITIVITY: f32 = 0.002;
pub const DEFAULT_SENSITIVITY: f32 = 1.0;
pub const MIN_SENSITIVITY: f32 = 0.25;
pub const MAX_SENSITIVITY: f32 = 3.0;
const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;
const SPAWN_PITCH: f32 = -15.0 * std::f32::consts::PI / 180.0;

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    sensitivity: f32,
    fov: f32,
}

impl Camera {
    pub fn spawn(position: Vec3) -> Self {
        Self {
            position,
            yaw: 0.0,
            pitch: SPAWN_PITCH,
            sensitivity: DEFAULT_SENSITIVITY,
            fov: DEFAULT_FOV,
        }
    }

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

    pub fn forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(cy * cp, sp, sy * cp)
    }

    pub fn set_sensitivity(&mut self, factor: f32) {
        if !factor.is_nan() {
            self.sensitivity = factor.clamp(MIN_SENSITIVITY, MAX_SENSITIVITY);
        }
    }

    pub fn sensitivity(&self) -> f32 {
        self.sensitivity
    }

    pub fn set_fov(&mut self, degrees: f32) {
        if !degrees.is_nan() {
            self.fov = degrees.clamp(MIN_FOV, MAX_FOV);
        }
    }

    pub fn fov(&self) -> f32 {
        self.fov
    }

    pub fn rotate(&mut self, dx: f32, dy: f32) {
        let step = BASE_SENSITIVITY * self.sensitivity;
        self.yaw = (self.yaw + dx * step) % std::f32::consts::TAU;
        self.pitch = (self.pitch - dy * step).clamp(-MAX_PITCH, MAX_PITCH);
    }

    pub fn view_proj(&self, aspect: f32, far: f32, fov_scale: f32) -> Mat4 {
        let fov = (self.fov * fov_scale).to_radians();
        let proj = directx::perspective(fov, aspect, NEAR, far);
        let view = view::look_to_mat4(self.position, self.forward(), Vec3::Y);
        proj * view
    }
}

#[cfg(test)]
mod tests;
