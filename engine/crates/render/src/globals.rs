use glam::{Mat3, Mat4, Vec2, Vec3};

/// Uniform `Globals` из `shader.wgsl`. Раскладка по правилам выравнивания WGSL
pub struct Globals {
    pub view_proj: Mat4,
    pub camera_pos: Vec3,
    pub fog_start: f32,
    pub fog_color: Vec3,
    pub fog_end: f32,
    /// 0 — воздух, 1 — вода, 2 — лава; enum в uniform WGSL запрещён.
    pub eye_fluid: f32,
    pub darkness: f32,
    pub anim_time: f32,
    pub view_dir: Mat4,
    pub sun_dir: Vec3,
    pub night: f32,
    pub zenith: Vec3,
    pub warm: f32,
    pub horizon_tilt: f32,
    pub star_rotation: Vec2,
}

pub fn view_dir_matrix(view_proj: Mat4) -> Mat4 {
    let row = |i: usize| {
        Vec3::new(
            view_proj.x_axis[i],
            view_proj.y_axis[i],
            view_proj.z_axis[i],
        )
    };
    let rows = Mat3::from_cols(row(0), row(1), row(3)).transpose();
    Mat4::from_mat3(rows.inverse())
}

impl Globals {
    pub const SIZE: usize = 224;

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let floats = self
            .view_proj
            .to_cols_array()
            .into_iter()
            .chain(self.camera_pos.to_array())
            .chain([self.fog_start])
            .chain(self.fog_color.to_array())
            .chain([
                self.fog_end,
                self.eye_fluid,
                self.darkness,
                self.anim_time,
                0.0,
            ])
            .chain(self.view_dir.to_cols_array())
            .chain(self.sun_dir.to_array())
            .chain([self.night])
            .chain(self.zenith.to_array())
            .chain([self.warm, self.horizon_tilt])
            .chain(self.star_rotation.to_array());
        let mut bytes = [0; Self::SIZE];
        for (chunk, f) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(floats) {
            *chunk = f.to_le_bytes();
        }
        bytes
    }
}

#[cfg(test)]
mod tests;
