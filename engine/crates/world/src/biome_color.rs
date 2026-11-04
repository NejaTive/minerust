use crate::biome::Biome;
use crate::textures::DecodedImage;

pub const BLEND_RADIUS: i32 = 2;
const IDS: usize = 256;

const GRASS_CORNERS: [[u8; 3]; 3] = [[0x80, 0xB4, 0x97], [0xBF, 0xB7, 0x55], [0x47, 0xCD, 0x33]];
const FOLIAGE_CORNERS: [[u8; 3]; 3] = [[0x60, 0xA1, 0x7B], [0xAE, 0xA4, 0x2A], [0x30, 0xBB, 0x0B]];
const SWAMP: [u8; 3] = [0x6A, 0x70, 0x39];
const SWAMP_DARK_GRASS: [u8; 3] = [0x4C, 0x76, 0x3C];
const SWAMP_WATER: [u8; 3] = [0xE0, 0xFF, 0xAE];
const SWAMP_NOISE_SCALE: f32 = 0.045;
const SWAMP_DARK_BELOW: f32 = -0.25;
const MESA_GRASS: [u8; 3] = [0x90, 0x81, 0x4D];
const MESA_FOLIAGE: [u8; 3] = [0x9E, 0x81, 0x4D];
const ROOFED_FOREST_GRASS_BONUS: [u8; 3] = [0x28, 0x34, 0x0A];

fn unit(rgb: [u8; 3]) -> [f32; 3] {
    rgb.map(|c| f32::from(c) / 255.0)
}

/// Тот же треугольник холодно / жарко и сухо / жарко и влажно, что и в colormap Minecraft.
fn gradient(corners: &[[u8; 3]; 3], temperature: f32, rainfall: f32) -> [u8; 3] {
    let t = temperature.clamp(0.0, 1.0);
    let wet = rainfall.clamp(0.0, 1.0) * t;
    std::array::from_fn(|c| {
        let [cold, dry, hot] = corners.map(|corner| f32::from(corner[c]));
        (cold + t * (dry - cold) + wet * (hot - dry)).round() as u8
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BiomeTint {
    #[default]
    None,
    Grass,
    Foliage,
    Water,
}

impl BiomeTint {
    pub fn code(self) -> f32 {
        match self {
            BiomeTint::None => 0.0,
            BiomeTint::Grass => 1.0,
            BiomeTint::Foliage => 2.0,
            BiomeTint::Water => 3.0,
        }
    }
}

pub struct Colormap {
    width: u32,
    height: u32,
    rgb: Vec<[u8; 3]>,
}

impl Colormap {
    pub fn from_image(image: &DecodedImage) -> Option<Colormap> {
        if image.width == 0 || image.height == 0 {
            return None;
        }
        let rgb = image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| [p[0], p[1], p[2]])
            .collect();
        Some(Colormap {
            width: image.width,
            height: image.height,
            rgb,
        })
    }

    pub fn at(&self, temperature: f32, downfall: f32) -> [u8; 3] {
        let t = temperature.clamp(0.0, 1.0);
        let d = downfall.clamp(0.0, 1.0) * t;
        let x = ((1.0 - t) * (self.width - 1) as f32).round() as usize;
        let y = ((1.0 - d) * (self.height - 1) as f32).round() as usize;
        self.rgb[y * self.width as usize + x]
    }
}

fn lattice(x: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (z as u32).wrapping_mul(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h >> 8) as f32 / (1 << 23) as f32 - 1.0
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Значение в [-1, 1], зависит только от мировых x, z; пятна порядка `1 / SWAMP_NOISE_SCALE` блоков.
fn swamp_noise(x: i32, z: i32) -> f32 {
    let (fx, fz) = (x as f32 * SWAMP_NOISE_SCALE, z as f32 * SWAMP_NOISE_SCALE);
    let (ix, iz) = (fx.floor(), fz.floor());
    let (tx, tz) = (smooth(fx - ix), smooth(fz - iz));
    let (ix, iz) = (ix as i32, iz as i32);
    let top = lattice(ix, iz) + tx * (lattice(ix + 1, iz) - lattice(ix, iz));
    let bottom = lattice(ix, iz + 1) + tx * (lattice(ix + 1, iz + 1) - lattice(ix, iz + 1));
    top + tz * (bottom - top)
}

/// `BiomeSwamp.getGrassColorAtPos`: два оттенка пятнами по мировой координате.
pub fn swamp_grass(x: i32, z: i32) -> [u8; 3] {
    if swamp_noise(x, z) < SWAMP_DARK_BELOW {
        SWAMP_DARK_GRASS
    } else {
        SWAMP
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct BiomePalette {
    grass: [[u8; 3]; IDS],
    foliage: [[u8; 3]; IDS],
    water: [[u8; 3]; IDS],
}

impl BiomePalette {
    pub fn constants() -> BiomePalette {
        Self::build(
            |b| gradient(&GRASS_CORNERS, b.temperature(), b.rainfall()),
            |b| gradient(&FOLIAGE_CORNERS, b.temperature(), b.rainfall()),
        )
    }

    pub fn from_colormaps(grass: Option<&Colormap>, foliage: Option<&Colormap>) -> BiomePalette {
        Self::build(
            |b| match grass {
                Some(map) => map.at(b.temperature(), b.rainfall()),
                None => gradient(&GRASS_CORNERS, b.temperature(), b.rainfall()),
            },
            |b| match foliage {
                Some(map) => map.at(b.temperature(), b.rainfall()),
                None => gradient(&FOLIAGE_CORNERS, b.temperature(), b.rainfall()),
            },
        )
    }

    fn build(grass: impl Fn(Biome) -> [u8; 3], foliage: impl Fn(Biome) -> [u8; 3]) -> BiomePalette {
        let mut palette = BiomePalette {
            grass: [[0; 3]; IDS],
            foliage: [[0; 3]; IDS],
            water: [[255; 3]; IDS],
        };
        for &biome in Biome::ALL {
            let slot = usize::from(biome.id());
            let (mut g, mut f) = (grass(biome), foliage(biome));
            match biome {
                Biome::Swampland | Biome::SwamplandM => (g, f) = (SWAMP, SWAMP),
                Biome::RoofedForest | Biome::RoofedForestM => {
                    g = std::array::from_fn(|c| {
                        ((u16::from(g[c] & 0xFE) + u16::from(ROOFED_FOREST_GRASS_BONUS[c])) >> 1)
                            as u8
                    });
                }
                _ if biome.is_mesa() => (g, f) = (MESA_GRASS, MESA_FOLIAGE),
                _ => {}
            }
            palette.grass[slot] = g;
            palette.foliage[slot] = f;
            if biome.is_swamp() {
                palette.water[slot] = SWAMP_WATER;
            }
        }
        palette
    }

    pub fn grass(&self, biome: Biome) -> [f32; 3] {
        unit(self.grass[usize::from(biome.id())])
    }

    pub fn foliage(&self, biome: Biome) -> [f32; 3] {
        unit(self.foliage[usize::from(biome.id())])
    }

    pub fn water(&self, biome: Biome) -> [f32; 3] {
        unit(self.water[usize::from(biome.id())])
    }

    pub fn plains(&self, kind: BiomeTint) -> [f32; 3] {
        match kind {
            BiomeTint::None | BiomeTint::Water => [1.0; 3],
            BiomeTint::Grass => self.grass(Biome::Plains),
            BiomeTint::Foliage => self.foliage(Biome::Plains),
        }
    }

    /// Цвета травы, листвы и воды колонки 16×16 для `Renderer::set_biome_column`: три слоя
    /// подряд, строки по z, по четыре байта RGBA на блок. `biomes` — квадрат со стороной
    /// `16 + 2 * BLEND_RADIUS` вокруг колонки, `(origin_x, origin_z)` — мировой угол самой
    /// колонки; цвет блока — среднее по его окну, у травы болота цвет клетки зависит от x, z.
    pub fn column_bytes(&self, biomes: &[Biome], origin_x: i32, origin_z: i32) -> Vec<u8> {
        let side = 16 + 2 * BLEND_RADIUS as usize;
        let window = (2 * BLEND_RADIUS + 1) as usize;
        let mut out = Vec::with_capacity(16 * 16 * 12);
        for layer in 0..3 {
            let cells: Vec<[u8; 3]> = biomes
                .iter()
                .enumerate()
                .map(|(i, biome)| {
                    let slot = usize::from(biome.id());
                    match layer {
                        0 if biome.is_swamp() => swamp_grass(
                            origin_x - BLEND_RADIUS + (i % side) as i32,
                            origin_z - BLEND_RADIUS + (i / side) as i32,
                        ),
                        0 => self.grass[slot],
                        1 => self.foliage[slot],
                        _ => self.water[slot],
                    }
                })
                .collect();
            for z in 0..16 {
                for x in 0..16 {
                    let mut sum = [0u32; 3];
                    for dz in 0..window {
                        for dx in 0..window {
                            let cell = cells[(z + dz) * side + x + dx];
                            for c in 0..3 {
                                sum[c] += u32::from(cell[c]);
                            }
                        }
                    }
                    let n = (window * window) as u32;
                    out.extend(sum.map(|v| ((v + n / 2) / n) as u8));
                    out.push(255);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests;
