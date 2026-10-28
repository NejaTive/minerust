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
mod tests {
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
            underwater: 108.0,
            darkness: 109.0,
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
        assert!(bytes[104..112].iter().all(|&b| b == 0));
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
            // Точка на этом луче проецируется обратно в тот же пиксель.
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
}
