use glam::{Mat4, Vec3, Vec4};

pub const MAX_CASCADES: usize = 3;
const SPLIT_LAMBDA: f32 = 0.7;
const MAX_CASTER_REACH: f32 = 512.0;
const MIN_LIGHT_ELEVATION: f32 = 0.04;
const LIGHT_STEP_COS: f32 = 0.999_998_5;

/// Направление света меняется ступенями (~0.1°): между ними карта и привязка к сетке
/// текселей неподвижны, края теней не плывут вместе со временем суток.
pub fn quantize_light_dir(previous: Option<Vec3>, dir: Vec3) -> Vec3 {
    match previous {
        Some(prev) if prev.dot(dir) >= LIGHT_STEP_COS => prev,
        _ => dir,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShadowQuality {
    Low,
    #[default]
    High,
}

impl ShadowQuality {
    pub fn resolution(self) -> u32 {
        match self {
            ShadowQuality::Low => 1024,
            ShadowQuality::High => 2048,
        }
    }

    pub fn cascades(self) -> usize {
        match self {
            ShadowQuality::Low => 2,
            ShadowQuality::High => 3,
        }
    }

    pub fn pcf_taps(self) -> u32 {
        match self {
            ShadowQuality::Low => 2,
            ShadowQuality::High => 3,
        }
    }
}

/// Направление на источник света и сила тени; ночью свет — луна (`-sun_dir`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowLight {
    pub dir: Vec3,
    pub strength: f32,
}

pub fn shadow_light(sun_dir: Vec3, warm: f32, rain: f32) -> Option<ShadowLight> {
    let elevation = sun_dir.y.abs();
    if elevation < MIN_LIGHT_ELEVATION {
        return None;
    }
    let sun_up = sun_dir.y > 0.0;
    let source = if sun_up { 1.0 - 0.35 * warm } else { 0.3 };
    let strength = smoothstep(0.02, 0.25, elevation) * source * (1.0 - 0.6 * rain);
    (strength > 0.01).then_some(ShadowLight {
        dir: if sun_up { sun_dir } else { -sun_dir },
        strength,
    })
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Clone, Copy, Debug)]
pub struct Cascades {
    pub count: usize,
    pub matrices: [Mat4; MAX_CASCADES],
    /// Дальняя граница каждого каскада — линейная глубина от камеры.
    pub splits: [f32; MAX_CASCADES],
    /// Размер текселя карты в блоках.
    pub texel_world: [f32; MAX_CASCADES],
}

/// Границы каскадов по практической схеме: смесь логарифмической и равномерной.
pub fn split_distances(near: f32, far: f32, count: usize) -> [f32; MAX_CASCADES] {
    let mut splits = [far; MAX_CASCADES];
    for (i, split) in splits.iter_mut().enumerate().take(count) {
        let fraction = (i + 1) as f32 / count as f32;
        let log = near * (far / near).powf(fraction);
        let uniform = near + (far - near) * fraction;
        *split = SPLIT_LAMBDA * log + (1.0 - SPLIT_LAMBDA) * uniform;
    }
    splits[count - 1] = far;
    splits
}

/// Углы среза пирамиды камеры между линейными глубинами `from` и `to`.
fn slice_corners(inverse: Mat4, near: f32, far: f32, from: f32, to: f32) -> [Vec3; 8] {
    let unproject = |x: f32, y: f32, z: f32| {
        let p = inverse * Vec4::new(x, y, z, 1.0);
        p.truncate() / p.w
    };
    let mut corners = [Vec3::ZERO; 8];
    for (i, corner) in corners.iter_mut().enumerate() {
        let x = if i & 1 == 0 { -1.0 } else { 1.0 };
        let y = if i & 2 == 0 { -1.0 } else { 1.0 };
        let ray_near = unproject(x, y, 0.0);
        let ray_far = unproject(x, y, 1.0);
        let depth = if i & 4 == 0 { from } else { to };
        *corner = ray_near.lerp(ray_far, (depth - near) / (far - near));
    }
    corners
}

fn light_basis(light_dir: Vec3) -> (Vec3, Vec3) {
    let up = if light_dir.y.abs() > 0.99 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let right = up.cross(light_dir).normalize();
    (right, light_dir.cross(right))
}

/// Ортопроекция одного каскада: сфера-обёртка среза, центр привязан к сетке текселей
/// в пространстве света, чтобы тени не дрожали при движении и повороте камеры.
fn cascade_matrix(
    corners: &[Vec3; 8],
    light_dir: Vec3,
    resolution: u32,
    world_top: f32,
) -> (Mat4, f32) {
    let center = corners.iter().sum::<Vec3>() / 8.0;
    let radius = corners
        .iter()
        .map(|c| c.distance(center))
        .fold(0.0, f32::max);
    // Запас на привязку центра к сетке: сфера сдвигается не больше чем на тексель.
    let radius = (radius * 16.0).ceil() / 16.0 * (1.0 + 4.0 / resolution as f32);
    let texel = 2.0 * radius / resolution as f32;

    let (right, up) = light_basis(light_dir);
    let snap = |v: f32| (v / texel).floor() * texel;
    let cx = snap(center.dot(right));
    let cy = snap(center.dot(up));
    let cz = center.dot(light_dir);

    let reach_to_top = if light_dir.y > 1e-3 {
        (world_top - center.y) / light_dir.y
    } else {
        MAX_CASTER_REACH
    };
    let reach = reach_to_top.clamp(0.0, MAX_CASTER_REACH) + radius;
    let z_max = cz + reach;
    let z_min = cz - radius;
    let depth = z_max - z_min;

    // x, y — в пространстве света, глубина 0..1 от источника к сцене (z_max → 0).
    let matrix = Mat4::from_cols(
        Vec4::new(right.x / radius, up.x / radius, -light_dir.x / depth, 0.0),
        Vec4::new(right.y / radius, up.y / radius, -light_dir.y / depth, 0.0),
        Vec4::new(right.z / radius, up.z / radius, -light_dir.z / depth, 0.0),
        Vec4::new(-cx / radius, -cy / radius, z_max / depth, 1.0),
    );
    (matrix, texel)
}

pub fn compute_cascades(
    view_proj: Mat4,
    near: f32,
    far: f32,
    light_dir: Vec3,
    quality: ShadowQuality,
    world_top: f32,
) -> Cascades {
    let count = quality.cascades();
    let splits = split_distances(near, far, count);
    let inverse = view_proj.inverse();
    let mut cascades = Cascades {
        count,
        matrices: [Mat4::ZERO; MAX_CASCADES],
        splits,
        texel_world: [0.0; MAX_CASCADES],
    };
    let mut from = near;
    for (i, &to) in splits.iter().enumerate().take(count) {
        let corners = slice_corners(inverse, near, far, from, to);
        let (matrix, texel) = cascade_matrix(&corners, light_dir, quality.resolution(), world_top);
        cascades.matrices[i] = matrix;
        cascades.texel_world[i] = texel;
        from = to;
    }
    cascades
}

pub struct ShadowData {
    pub cascades: Cascades,
    pub light: ShadowLight,
    pub quality: ShadowQuality,
    pub cloud_strength: f32,
}

impl ShadowData {
    pub const SIZE: usize = 256;

    pub fn cloud_only(light: ShadowLight, cloud_strength: f32) -> Self {
        Self {
            cascades: Cascades {
                count: 0,
                matrices: [Mat4::ZERO; MAX_CASCADES],
                splits: [0.0; MAX_CASCADES],
                texel_world: [0.0; MAX_CASCADES],
            },
            light,
            quality: ShadowQuality::default(),
            cloud_strength,
        }
    }

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let c = &self.cascades;
        let floats = c
            .matrices
            .iter()
            .flat_map(|m| m.to_cols_array())
            .chain(c.splits)
            .chain([0.0])
            .chain(self.light.dir.to_array())
            .chain([self.light.strength])
            .chain(c.texel_world)
            .chain([0.0])
            .chain([
                c.count as f32,
                self.quality.pcf_taps() as f32,
                1.0 / self.quality.resolution() as f32,
                self.cloud_strength,
            ]);
        let mut bytes = [0; Self::SIZE];
        for (chunk, f) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(floats) {
            *chunk = f.to_le_bytes();
        }
        bytes
    }
}

#[cfg(test)]
mod tests;
