use glam::{IVec3, Vec3};

/// Попадание луча в блок.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// Твёрдый блок, в который попал луч.
    pub block: IVec3,
    /// Нормаль грани попадания: единичный вектор по одной оси, смотрит из блока к лучу.
    pub normal: IVec3,
    /// От `origin` до точки входа в блок, блоков.
    pub distance: f32,
}

/// DDA по сетке блоков (Amanatides—Woo): луч проходит воксели по очереди, без пропусков.
/// `dir` не обязан быть нормирован. Воксель, в котором лежит `origin`, пропускается —
/// при полёте сквозь рельеф камера бывает внутри блока. Возвращает первый воксель,
/// где `is_solid` истинно, не дальше `max_distance` блоков.
pub fn raycast(
    origin: Vec3,
    dir: Vec3,
    max_distance: f32,
    is_solid: impl Fn(IVec3) -> bool,
) -> Option<Hit> {
    // Нулевой или нечисловой `dir` превращается в ноль; с NaN в origin или дальности
    // цикл ниже никогда бы не закончился.
    let dir = dir.normalize_or_zero();
    if dir == Vec3::ZERO || !origin.is_finite() || max_distance.is_nan() {
        return None;
    }
    let mut voxel = origin.floor().as_ivec3();
    let step = IVec3::new(sign(dir.x), sign(dir.y), sign(dir.z));
    // Расстояние вдоль луча между соседними границами по оси; по оси без движения — бесконечность.
    let t_delta = dir.abs().recip();
    // Расстояние до первой границы по каждой оси.
    let mut t_max = Vec3::from_array(std::array::from_fn(|axis| {
        let to_border = if dir[axis] > 0.0 {
            voxel[axis] as f32 + 1.0 - origin[axis]
        } else {
            origin[axis] - voxel[axis] as f32
        };
        to_border * t_delta[axis]
    }));

    loop {
        // При равенстве (луч точно в ребро или угол) шагаем по первой оси: проходим
        // через одного из соседей по ребру, поэтому между двумя блоками луч не проскочит.
        let axis = if t_max.x <= t_max.y && t_max.x <= t_max.z {
            0
        } else if t_max.y <= t_max.z {
            1
        } else {
            2
        };
        let distance = t_max[axis];
        if distance > max_distance {
            return None;
        }
        voxel[axis] += step[axis];
        t_max[axis] += t_delta[axis];
        if is_solid(voxel) {
            let mut normal = IVec3::ZERO;
            normal[axis] = -step[axis];
            return Some(Hit {
                block: voxel,
                normal,
                distance,
            });
        }
    }
}

fn sign(v: f32) -> i32 {
    if v > 0.0 {
        1
    } else if v < 0.0 {
        -1
    } else {
        0
    }
}

#[cfg(test)]
mod tests;
