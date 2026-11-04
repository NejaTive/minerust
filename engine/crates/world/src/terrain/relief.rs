use crate::biome::Biome;
use crate::column::WORLD_HEIGHT;
use crate::noise::Octaves;

const GRID: usize = 5;
const CELL_XZ: usize = 4;
const CELL_Y: usize = 8;
const LEVELS: usize = (WORLD_HEIGHT as usize) / CELL_Y + 1;

const COORDINATE_SCALE: f64 = 684.412;
const HEIGHT_SCALE: f64 = 684.412;
const MAIN_SCALE: (f64, f64, f64) = (80.0, 160.0, 80.0);
const DEPTH_SCALE: f64 = 200.0;
const DEPTH_SAMPLE_Y: f64 = 10.0;
const LIMIT_SCALE: f64 = 512.0;
const BASE_SIZE: f64 = 8.5;
const STRETCH_Y: f64 = 12.0;
const TOP_FADE_CELLS: f64 = 3.0;
const TOP_FADE_TARGET: f64 = -10.0;

const NOISE_BOUND: f64 = 70.0;
const LIMIT_OCTAVES: usize = 16;
const LIMIT_SKIPPED: usize = 6;
const MAIN_OCTAVES: usize = 8;
const DEPTH_OCTAVES: usize = 16;
const SURFACE_OCTAVES: usize = 4;
const SURFACE_FREQUENCY: f64 = 0.0625;
const SURFACE_AMPLITUDE: f64 = 0.55;
const TEMPERATURE_SCALE: f64 = 8.0;
const TEMPERATURE_COOLING: f32 = 0.05 / 30.0;
const COOLING_FROM_Y: i32 = 64;

const SALT_LOW: u64 = 0x4C4F;
const SALT_HIGH: u64 = 0x4849;
const SALT_MAIN: u64 = 0x4D41;
const SALT_DEPTH: u64 = 0x4445;
const SALT_SURFACE: u64 = 0x5355;
const SALT_TEMPERATURE: u64 = 0x5445;

#[derive(Clone, Copy, Default)]
pub(super) struct Solid([u128; 2]);

impl Solid {
    pub fn get(self, y: i32) -> bool {
        self.0[(y >> 7) as usize] >> (y & 127) & 1 == 1
    }

    pub fn set(&mut self, y: i32) {
        self.0[(y >> 7) as usize] |= 1 << (y & 127);
    }

    pub fn highest(self) -> Option<i32> {
        match self.0 {
            [_, high] if high != 0 => Some(255 - high.leading_zeros() as i32),
            [low, _] if low != 0 => Some(127 - low.leading_zeros() as i32),
            _ => None,
        }
    }
}

pub(super) struct Relief {
    low: Octaves,
    high: Octaves,
    main: Octaves,
    depth: Octaves,
    surface: Octaves,
    temperature: Octaves,
}

impl Relief {
    pub fn new(seed: i32) -> Self {
        let seed = u64::from(seed as u32) << 16;
        Self {
            low: Octaves::new(seed ^ SALT_LOW, LIMIT_OCTAVES, LIMIT_SKIPPED),
            high: Octaves::new(seed ^ SALT_HIGH, LIMIT_OCTAVES, LIMIT_SKIPPED),
            main: Octaves::new(seed ^ SALT_MAIN, MAIN_OCTAVES, 0),
            depth: Octaves::new(seed ^ SALT_DEPTH, DEPTH_OCTAVES, 0),
            surface: Octaves::new(seed ^ SALT_SURFACE, SURFACE_OCTAVES, 0),
            temperature: Octaves::new(seed ^ SALT_TEMPERATURE, 1, 0),
        }
    }

    /// `NoiseGeneratorPerlin` слоя `Biome.generateBiomeTerrain`: толщина верхнего слоя.
    pub fn surface_noise(&self, x: i32, z: i32) -> f64 {
        self.surface.sample(
            f64::from(x) * SURFACE_FREQUENCY,
            0.0,
            f64::from(z) * SURFACE_FREQUENCY,
        ) * SURFACE_AMPLITUDE
    }

    /// `Biome.getTemperature(pos)`: выше y=64 воздух холоднее.
    pub fn temperature(&self, biome: Biome, x: i32, y: i32, z: i32) -> f32 {
        if y <= COOLING_FROM_Y {
            return biome.temperature();
        }
        let noise = self
            .temperature
            .sample(f64::from(x) / 8.0, 0.0, f64::from(z) / 8.0)
            * TEMPERATURE_SCALE;
        biome.temperature() - (noise as f32 + (y - COOLING_FROM_Y) as f32) * TEMPERATURE_COOLING
    }

    /// Биты твёрдых блоков каждой колонки 16×16 (индекс `z * 16 + x`, бит `y`):
    /// `ChunkGeneratorOverworld.generateHeightmap` + `setBlocksInChunk`.
    /// `cell(i, j)` — биом грубой карты, окно сдвинуто на 2 клетки от узла сетки.
    pub fn solid(
        &self,
        column_x: i32,
        column_z: i32,
        cell: &impl Fn(usize, usize) -> Biome,
    ) -> [Solid; 256] {
        let mut field = [[[0.0f64; GRID]; LEVELS]; GRID];
        for (i, plane) in field.iter_mut().enumerate() {
            for k in 0..GRID {
                let gx = f64::from(column_x * CELL_XZ as i32 + i as i32);
                let gz = f64::from(column_z * CELL_XZ as i32 + k as i32);
                let (offset, scale) = self.shape(gx, gz, |dx, dz| cell(i + dx, k + dz));
                for (level, row) in plane.iter_mut().enumerate() {
                    row[k] = self.density(gx, level as f64, gz, offset, scale);
                }
            }
        }
        interpolate(&field)
    }

    fn shape(&self, gx: f64, gz: f64, near: impl Fn(usize, usize) -> Biome) -> (f64, f64) {
        let center = near(2, 2).base_height();
        let (mut scale, mut depth, mut total) = (0.0f64, 0.0f64, 0.0f64);
        for dz in 0..GRID {
            for dx in 0..GRID {
                let biome = near(dx, dz);
                let (ox, oz) = (dx as f64 - 2.0, dz as f64 - 2.0);
                let base = f64::from(biome.base_height());
                let mut weight = 10.0 / (ox * ox + oz * oz + 0.2).sqrt() / (base + 2.0);
                if biome.base_height() > center {
                    weight /= 2.0;
                }
                scale += f64::from(biome.height_variation()) * weight;
                depth += base * weight;
                total += weight;
            }
        }
        let scale = scale / total * 0.9 + 0.1;
        let depth = (depth / total * 4.0 - 1.0) / 8.0;

        let mut noise = self
            .depth
            .sample(gx * DEPTH_SCALE, DEPTH_SAMPLE_Y, gz * DEPTH_SCALE)
            / 8000.0;
        if noise < 0.0 {
            noise = -noise * 0.3;
        }
        noise = noise * 3.0 - 2.0;
        noise = if noise < 0.0 {
            (noise / 2.0).max(-1.0) / 1.4 / 2.0
        } else {
            noise.min(1.0) / 8.0
        };
        let offset = (depth + noise * 0.2) * BASE_SIZE / 8.0;
        (BASE_SIZE + offset * 4.0, scale)
    }

    fn density(&self, gx: f64, level: f64, gz: f64, base: f64, scale: f64) -> f64 {
        let mut slope = (level - base) * STRETCH_Y * 128.0 / 256.0 / scale;
        if slope < 0.0 {
            slope *= 4.0;
        }
        let density = if slope.abs() > NOISE_BOUND {
            -slope
        } else {
            self.noise_density(gx, level, gz) - slope
        };
        let fade_from = (LEVELS - 1) as f64 - TOP_FADE_CELLS;
        if level > fade_from {
            let t = (level - fade_from) / TOP_FADE_CELLS;
            return density * (1.0 - t) + TOP_FADE_TARGET * t;
        }
        density
    }

    fn noise_density(&self, gx: f64, level: f64, gz: f64) -> f64 {
        let main = self.main.sample(
            gx * COORDINATE_SCALE / MAIN_SCALE.0,
            level * HEIGHT_SCALE / MAIN_SCALE.1,
            gz * COORDINATE_SCALE / MAIN_SCALE.2,
        );
        let blend = ((main / 10.0 + 1.0) / 2.0).clamp(0.0, 1.0);
        let limit = |octaves: &Octaves| {
            octaves.sample(
                gx * COORDINATE_SCALE,
                level * HEIGHT_SCALE,
                gz * COORDINATE_SCALE,
            ) / LIMIT_SCALE
        };
        match blend {
            0.0 => limit(&self.low),
            1.0 => limit(&self.high),
            _ => {
                let (lower, upper) = (limit(&self.low), limit(&self.high));
                lower + (upper - lower) * blend
            }
        }
    }
}

fn interpolate(field: &[[[f64; GRID]; LEVELS]; GRID]) -> [Solid; 256] {
    let mut solid = [Solid::default(); 256];
    let lerp = |t: f64, a: f64, b: f64| a + (b - a) * t;
    for cx in 0..CELL_XZ {
        for cz in 0..CELL_XZ {
            for cy in 0..LEVELS - 1 {
                let corner = |dx: usize, dy: usize, dz: usize| field[cx + dx][cy + dy][cz + dz];
                for dy in 0..CELL_Y {
                    let ty = dy as f64 / CELL_Y as f64;
                    let y = cy * CELL_Y + dy;
                    let low_low = lerp(ty, corner(0, 0, 0), corner(0, 1, 0));
                    let high_low = lerp(ty, corner(1, 0, 0), corner(1, 1, 0));
                    let low_high = lerp(ty, corner(0, 0, 1), corner(0, 1, 1));
                    let high_high = lerp(ty, corner(1, 0, 1), corner(1, 1, 1));
                    for dx in 0..CELL_XZ {
                        let tx = dx as f64 / CELL_XZ as f64;
                        let near = lerp(tx, low_low, high_low);
                        let far = lerp(tx, low_high, high_high);
                        for dz in 0..CELL_XZ {
                            let tz = dz as f64 / CELL_XZ as f64;
                            if lerp(tz, near, far) > 0.0 {
                                let index = (cz * CELL_XZ + dz) * 16 + cx * CELL_XZ + dx;
                                solid[index].set(y as i32);
                            }
                        }
                    }
                }
            }
        }
    }
    solid
}
