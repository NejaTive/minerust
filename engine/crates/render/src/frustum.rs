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
mod tests;
