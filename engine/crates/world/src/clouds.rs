use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};

pub const CLOUD_MAP: usize = 256;
pub const CLOUD_CELL: i32 = 12;
pub const CLOUD_Y: i32 = 160;
pub const CLOUD_HEIGHT: i32 = 4;
pub const CLOUD_DRIFT_SPEED: f64 = 1.0;

pub fn cloud_drift(seconds: f64) -> f32 {
    let period = (CLOUD_MAP as i32 * CLOUD_CELL) as f64;
    (seconds * CLOUD_DRIFT_SPEED).rem_euclid(period) as f32
}

const COVERAGE: f32 = 0.35;
const NOISE_FREQUENCY: f32 = 0.07;
const SEED_OFFSET: i32 = 0x636c_6f75;

pub struct CloudMask {
    cells: Vec<u8>,
}

impl CloudMask {
    pub fn new(seed: i32) -> Self {
        let mut noise = FastNoiseLite::with_seed(seed.wrapping_add(SEED_OFFSET));
        noise.set_noise_type(Some(NoiseType::OpenSimplex2));
        noise.set_fractal_type(Some(FractalType::FBm));
        noise.set_fractal_octaves(Some(3));
        noise.set_frequency(Some(NOISE_FREQUENCY));

        let size = CLOUD_MAP as f32;
        let sample = |x: f32, z: f32| noise.get_noise_2d(x, z);
        let values: Vec<f32> = (0..CLOUD_MAP * CLOUD_MAP)
            .map(|i| {
                let (x, z) = ((i % CLOUD_MAP) as f32, (i / CLOUD_MAP) as f32);
                let (wx, wz) = (x / size, z / size);
                let (wx, wz) = (wx * wx * (3.0 - 2.0 * wx), wz * wz * (3.0 - 2.0 * wz));
                let low = sample(x, z) * (1.0 - wx) + sample(x - size, z) * wx;
                let high = sample(x, z - size) * (1.0 - wx) + sample(x - size, z - size) * wx;
                low * (1.0 - wz) + high * wz
            })
            .collect();

        let mut sorted = values.clone();
        sorted.sort_unstable_by(f32::total_cmp);
        let threshold = sorted[((1.0 - COVERAGE) * sorted.len() as f32) as usize];
        Self {
            cells: values.iter().map(|&v| u8::from(v >= threshold)).collect(),
        }
    }

    pub fn cells(&self) -> &[u8] {
        &self.cells
    }

    pub fn get(&self, x: i32, z: i32) -> bool {
        let wrap = |v: i32| v.rem_euclid(CLOUD_MAP as i32) as usize;
        self.cells[wrap(z) * CLOUD_MAP + wrap(x)] != 0
    }
}

#[cfg(test)]
mod tests;
