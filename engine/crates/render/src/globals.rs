use glam::{Mat3, Mat4, Vec2, Vec3};

/// Uniform `Globals` из `shader.wgsl`. Раскладка по правилам выравнивания WGSL
/// для uniform: vec3 выровнен на 16 байт, а следующий за ним f32 занимает его хвост.
/// После `darkness` 8 байт пустоты: mat4 начинается с кратного 16 смещения.
pub struct Globals {
    pub view_proj: Mat4,
    pub camera_pos: Vec3,
    /// Горизонтальное расстояние в блоках, с которого начинается туман.
    pub fog_start: f32,
    /// Линейный цвет — тот же, что у неба.
    pub fog_color: Vec3,
    /// Горизонтальное расстояние в блоках, с которого виден только туман.
    pub fog_end: f32,
    /// 1, если глаза под водой (тон и туман по полному расстоянию), иначе 0. f32, а не
    /// bool: bool в uniform WGSL запрещён.
    pub underwater: f32,
    /// Уровней солнечного света, которые отнимает ночь: 0 днём (`daytime::sky_state`).
    pub darkness: f32,
    /// Пиксель неба → направление взгляда в мире, см. [`view_dir_matrix`].
    pub view_dir: Mat4,
    /// Единичное направление на солнце (`daytime::sun_direction`).
    pub sun_dir: Vec3,
    /// Степень ночи 0..1: видимость звёзд.
    pub night: f32,
    /// Линейный цвет неба в зените; у горизонта небо цвета `fog_color`.
    pub zenith: Vec3,
    /// Вес тёплого пятна заката у горизонта 0..1.
    pub warm: f32,
    /// Насколько край земли ниже горизонта: тангенс угла (высота глаз над водой / дальность
    /// тумана), 0 на уровне воды и ниже. Небо отсчитывает градиент от этого края.
    pub horizon_tilt: f32,
    /// (cos, sin) поворота звёзд вокруг оси z (`daytime::star_rotation`).
    pub star_rotation: Vec2,
}

/// Матрица, переводящая `(ndc.x, ndc.y, 1, 0)` в направление взгляда пикселя в мире
/// (не нормированное). Берётся только линейная часть `view_proj` без переноса: так
/// позиция камеры не влияет и нет потери точности далеко от начала координат.
/// Направление `d` даёт `clip.xy = ndc * clip.w`, то есть `(row_x - ndc.x * row_w) · d = 0`
/// и так же для y; обратная к матрице строк (x, y, w) при `w = 1` решает систему.
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
            .chain([self.fog_end, self.underwater, self.darkness, 0.0, 0.0])
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
