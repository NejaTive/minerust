//! Туман и дальняя плоскость камеры — производные дальности прорисовки.

use crate::column::WORLD_HEIGHT;
use crate::section::SECTION_SIZE;

/// Доля `fog_end`, с которой начинается туман: ближе рельеф чистый, дальше плавно
/// уходит в цвет неба.
const FOG_START_FRACTION: f32 = 0.65;

/// Горизонтальные расстояния тумана в блоках: до `start` тумана нет, от `end` — только он.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fog {
    pub start: f32,
    pub end: f32,
}

impl Fog {
    /// Туман для дальности `radius` колонок.
    ///
    /// Загрузчик мешует колонки, у которых ключ отстоит от ключа колонки камеры не
    /// дальше `radius`, так что ближайшая колонка без меша смещена на `o` с
    /// |o| ≥ √(radius² + 1). Камера стоит где угодно в своей колонке, и по каждой оси
    /// до той колонки остаётся не меньше (|oᵢ| − 1) колонок: вектор этих зазоров
    /// отличается от `o` на вектор с компонентами из 0..1, длиной не больше √2.
    /// Отсюда расстояние до края мешей ≥ 16 · (√(radius² + 1) − √2), и туман, густой
    /// к этому расстоянию, скрывает край в любой точке колонки. Для 8 колонок это
    /// 106 блоков при точном минимуме 107.
    pub fn for_radius(radius: i32) -> Self {
        let r = radius as f32;
        let end = SECTION_SIZE as f32 * ((r * r + 1.0).sqrt() - std::f32::consts::SQRT_2);
        Self {
            start: end * FOG_START_FRACTION,
            end,
        }
    }

    /// Дальняя плоскость для камеры на высоте `camera_y`: всё, что ближе `end` по
    /// горизонтали и лежит в высоте мира, должно в неё влезать, иначе рельеф обрежется
    /// раньше, чем растворится. Глубина вдоль взгляда не больше евклидова расстояния,
    /// а оно не больше √(end² + dy²), где dy — до дальней от камеры границы мира по
    /// высоте. Блок запаса — на округления.
    pub fn far_plane(&self, camera_y: f32) -> f32 {
        let height = WORLD_HEIGHT as f32;
        let dy = camera_y.max(height - camera_y);
        (self.end * self.end + dy * dy).sqrt() + 1.0
    }
}

#[cfg(test)]
mod tests {
    use glam::{IVec2, Vec2, Vec3};

    use super::*;

    /// Расстояние от точки до квадрата колонки `key` по горизонтали.
    fn distance_to_column(point: Vec2, key: IVec2) -> f32 {
        let size = SECTION_SIZE as f32;
        let min = key.as_vec2() * size;
        point.clamp(min, min + size).distance(point)
    }

    #[test]
    fn mesh_edge_is_fully_fogged_from_any_point_of_camera_column() {
        let size = SECTION_SIZE as f32;
        for radius in 4..=16 {
            let fog = Fog::for_radius(radius);
            let reach = radius + 2;
            let mut nearest = f32::INFINITY;
            // Расстояние до колонки — выпуклая функция точки, так что минимум по
            // колонке камеры достигается в её углах.
            for corner in [Vec2::ZERO, Vec2::X, Vec2::Y, Vec2::ONE] {
                for z in -reach..=reach {
                    for x in -reach..=reach {
                        let key = IVec2::new(x, z);
                        if key.length_squared() > radius * radius {
                            nearest = nearest.min(distance_to_column(corner * size, key));
                        }
                    }
                }
            }
            assert!(
                fog.end <= nearest,
                "radius {radius}: {} > {nearest}",
                fog.end
            );
            // Оценка не слишком осторожна: туман не съедает лишнюю колонку.
            assert!(
                nearest - fog.end < size,
                "radius {radius}: {} vs {nearest}",
                fog.end
            );
            assert!(fog.start < fog.end);
        }
    }

    #[test]
    fn far_plane_covers_everything_before_fog_end() {
        let fog = Fog::for_radius(16);
        let height = WORLD_HEIGHT as f32;
        for camera_y in [-20.0, 0.0, 40.0, 64.0, 127.0, 300.0] {
            let far = fog.far_plane(camera_y);
            let camera = Vec3::new(0.0, camera_y, 0.0);
            for y in [0.0, height] {
                let edge = Vec3::new(fog.end, y, 0.0);
                assert!(
                    camera.distance(edge) < far,
                    "camera y {camera_y}, point y {y}"
                );
            }
        }
    }
}
