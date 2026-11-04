use crate::trees::splitmix64;

const SALT_OCTAVE: u64 = 0x9E37_79B9_7F4A_7C15;

struct Perlin {
    perm: [u8; 512],
    offset: [f64; 3],
}

impl Perlin {
    fn new(seed: u64) -> Self {
        let mut state = seed;
        let mut next = move || {
            state = state.wrapping_add(SALT_OCTAVE);
            splitmix64(state)
        };
        let mut offset = [0.0; 3];
        for slot in &mut offset {
            *slot = (next() >> 11) as f64 / (1u64 << 53) as f64 * 256.0;
        }
        let mut table: [u8; 256] = std::array::from_fn(|i| i as u8);
        for i in (1..256).rev() {
            table.swap(i, (next() % (i as u64 + 1)) as usize);
        }
        Self {
            perm: std::array::from_fn(|i| table[i & 255]),
            offset,
        }
    }

    fn noise(&self, x: f64, y: f64, z: f64) -> f64 {
        let (x, y, z) = (x + self.offset[0], y + self.offset[1], z + self.offset[2]);
        let (fx, fy, fz) = (x.floor(), y.floor(), z.floor());
        let (xi, yi, zi) = (
            (fx as i64 & 255) as usize,
            (fy as i64 & 255) as usize,
            (fz as i64 & 255) as usize,
        );
        let (x, y, z) = (x - fx, y - fy, z - fz);
        let (u, v, w) = (fade(x), fade(y), fade(z));
        let p = &self.perm;
        let a = p[xi] as usize + yi;
        let aa = p[a] as usize + zi;
        let ab = p[a + 1] as usize + zi;
        let b = p[xi + 1] as usize + yi;
        let ba = p[b] as usize + zi;
        let bb = p[b + 1] as usize + zi;
        lerp(
            w,
            lerp(
                v,
                lerp(u, grad(p[aa], x, y, z), grad(p[ba], x - 1.0, y, z)),
                lerp(
                    u,
                    grad(p[ab], x, y - 1.0, z),
                    grad(p[bb], x - 1.0, y - 1.0, z),
                ),
            ),
            lerp(
                v,
                lerp(
                    u,
                    grad(p[aa + 1], x, y, z - 1.0),
                    grad(p[ba + 1], x - 1.0, y, z - 1.0),
                ),
                lerp(
                    u,
                    grad(p[ab + 1], x, y - 1.0, z - 1.0),
                    grad(p[bb + 1], x - 1.0, y - 1.0, z - 1.0),
                ),
            ),
        )
    }
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

fn grad(hash: u8, x: f64, y: f64, z: f64) -> f64 {
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 {
        y
    } else if h == 12 || h == 14 {
        x
    } else {
        z
    };
    (if h & 1 == 0 { u } else { -u }) + (if h & 2 == 0 { v } else { -v })
}

/// Сумма октав Перлина как `NoiseGeneratorOctaves` 1.12: октава `j` берётся с частотой `2^-j`
/// от базовой и амплитудой `2^j`; нижние `skipped` октав (выше частоты сетки) не считаются.
pub struct Octaves {
    layers: Vec<(Perlin, f64, f64)>,
}

impl Octaves {
    pub fn new(seed: u64, count: usize, skipped: usize) -> Self {
        let layers = (skipped..count)
            .map(|j| {
                let perlin =
                    Perlin::new(splitmix64(seed ^ (j as u64 + 1).wrapping_mul(SALT_OCTAVE)));
                (perlin, 0.5f64.powi(j as i32), 2f64.powi(j as i32))
            })
            .collect();
        Self { layers }
    }

    pub fn sample(&self, x: f64, y: f64, z: f64) -> f64 {
        self.layers
            .iter()
            .map(|(perlin, frequency, amplitude)| {
                perlin.noise(x * frequency, y * frequency, z * frequency) * amplitude
            })
            .sum()
    }
}

#[cfg(test)]
mod tests;
