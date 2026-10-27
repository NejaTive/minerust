use glam::{Mat4, Vec3};

/// Uniform `Globals` из `shader.wgsl`. Раскладка совпадает и в WGSL, и в std140
/// для WebGL2: vec3 выровнен на 16 байт, а следующий за ним f32 занимает его хвост.
/// Размер структуры в обоих округляется до 16 байт: за `underwater` 12 байт хвоста.
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
}

impl Globals {
    pub const SIZE: usize = 112;

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let floats = self
            .view_proj
            .to_cols_array()
            .into_iter()
            .chain(self.camera_pos.to_array())
            .chain([self.fog_start])
            .chain(self.fog_color.to_array())
            .chain([self.fog_end, self.underwater]);
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
        };
        let bytes = globals.to_bytes();
        assert_eq!(bytes.len(), 112);
        assert_eq!(f32_at(&bytes, 0), 0.0);
        assert_eq!(f32_at(&bytes, 60), 15.0);
        assert_eq!(f32_at(&bytes, 64), 100.0);
        assert_eq!(f32_at(&bytes, 72), 102.0);
        assert_eq!(f32_at(&bytes, 76), 103.0);
        assert_eq!(f32_at(&bytes, 80), 104.0);
        assert_eq!(f32_at(&bytes, 88), 106.0);
        assert_eq!(f32_at(&bytes, 92), 107.0);
        assert_eq!(f32_at(&bytes, 96), 108.0);
        assert!(bytes[100..].iter().all(|&b| b == 0));
    }
}
