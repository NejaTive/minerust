use glam::{Mat4, Vec3, Vec4};

/// Пирамида видимости: 6 плоскостей `(n, d)`, внутри — где `n·p + d ≥ 0`.
/// Нормали не нормированы: для проверки знака длина не нужна.
pub struct Frustum {
    planes: [Vec4; 6],
}

impl Frustum {
    /// Плоскости по методу Гриба—Хартманна из матрицы view-projection.
    ///
    /// Точка видна, если в clip-пространстве −w ≤ x ≤ w, −w ≤ y ≤ w и 0 ≤ z ≤ w:
    /// у wgpu (и `directx::perspective`) глубина NDC 0..1, поэтому ближняя плоскость —
    /// сама строка 2, а не строка 3 + строка 2, как в OpenGL с глубиной −1..1.
    pub fn from_view_proj(m: Mat4) -> Self {
        let [r0, r1, r2, r3] = [m.row(0), m.row(1), m.row(2), m.row(3)];
        Self {
            planes: [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2],
        }
    }

    /// Пересекает ли коробка пирамиду. Консервативно: коробку у ребра пирамиды,
    /// но вне её, может посчитать видимой; видимую невидимой не считает никогда.
    pub fn intersects_aabb(&self, min: Vec3, max: Vec3) -> bool {
        self.planes.iter().all(|plane| {
            let normal = plane.truncate();
            // Угол коробки дальше всех по нормали (p-vertex): если и он снаружи,
            // снаружи вся коробка.
            let p = Vec3::select(normal.cmpge(Vec3::ZERO), max, min);
            normal.dot(p) + plane.w >= 0.0
        })
    }
}

#[cfg(test)]
mod tests {
    use glam::camera::rh::{proj::directx, view};

    use super::*;

    const NEAR: f32 = 0.1;
    const FAR: f32 = 100.0;

    /// Камера в начале координат смотрит вдоль −Z, поле зрения 90° по вертикали,
    /// квадратный экран: граница пирамиды проходит по |x| = |z| и |y| = |z|.
    fn frustum() -> Frustum {
        let proj = directx::perspective(std::f32::consts::FRAC_PI_2, 1.0, NEAR, FAR);
        let view = view::look_to_mat4(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
        Frustum::from_view_proj(proj * view)
    }

    fn cube(center: Vec3, half: f32) -> (Vec3, Vec3) {
        (center - half, center + half)
    }

    fn visible((min, max): (Vec3, Vec3)) -> bool {
        frustum().intersects_aabb(min, max)
    }

    #[test]
    fn box_in_front_is_visible() {
        assert!(visible(cube(Vec3::new(0.0, 0.0, -10.0), 1.0)));
    }

    #[test]
    fn box_behind_is_hidden() {
        assert!(!visible(cube(Vec3::new(0.0, 0.0, 10.0), 1.0)));
    }

    #[test]
    fn box_beside_view_is_hidden() {
        // На глубине 10 видно |x| ≤ 10.
        assert!(!visible(cube(Vec3::new(20.0, 0.0, -10.0), 1.0)));
        assert!(!visible(cube(Vec3::new(0.0, -20.0, -10.0), 1.0)));
    }

    #[test]
    fn box_beyond_far_plane_is_hidden() {
        assert!(!visible(cube(Vec3::new(0.0, 0.0, -FAR - 2.0), 1.0)));
        assert!(visible(cube(Vec3::new(0.0, 0.0, -FAR + 0.5), 1.0)));
    }

    #[test]
    fn box_just_before_near_plane_is_hidden() {
        let min = Vec3::new(-0.01, -0.01, -NEAR * 0.5);
        let max = Vec3::new(0.01, 0.01, -NEAR * 0.1);
        assert!(!visible((min, max)));
    }

    #[test]
    fn box_around_camera_is_visible() {
        assert!(visible(cube(Vec3::ZERO, 5.0)));
    }

    #[test]
    fn box_crossing_frustum_edge_is_visible() {
        // Центр снаружи (x = 12 при границе 10), но край заходит внутрь.
        assert!(visible(cube(Vec3::new(12.0, 0.0, -10.0), 3.0)));
        // Пересекает и ближнюю плоскость, и боковую.
        assert!(visible(cube(Vec3::new(1.0, 0.0, 0.0), 1.0)));
    }
}
