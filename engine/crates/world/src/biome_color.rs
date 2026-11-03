use crate::terrain::BiomeWeights;
use crate::textures::DecodedImage;

pub const BIOME_COUNT: usize = 4;

/// Порядок как в `BiomeWeights`: равнины, пустыня, лес, тайга. Тайга у нас всегда заснежена,
/// поэтому берётся климат Snowy Taiga.
const CLIMATE: [(f32, f32); BIOME_COUNT] = [(0.8, 0.4), (2.0, 0.0), (0.7, 0.8), (-0.5, 0.4)];

const GRASS: [u32; BIOME_COUNT] = [0x91BD59, 0xBFB755, 0x79C05A, 0x80B497];
const FOLIAGE: [u32; BIOME_COUNT] = [0x77AB2F, 0xAEA42A, 0x59AE30, 0x60A17B];

const fn rgb(hex: u32) -> [f32; 3] {
    [
        ((hex >> 16) & 0xFF) as f32 / 255.0,
        ((hex >> 8) & 0xFF) as f32 / 255.0,
        (hex & 0xFF) as f32 / 255.0,
    ]
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BiomeTint {
    #[default]
    None,
    Grass,
    Foliage,
}

impl BiomeTint {
    pub fn code(self) -> f32 {
        match self {
            BiomeTint::None => 0.0,
            BiomeTint::Grass => 1.0,
            BiomeTint::Foliage => 2.0,
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

    pub fn at(&self, temperature: f32, downfall: f32) -> [f32; 3] {
        let t = temperature.clamp(0.0, 1.0);
        let d = downfall.clamp(0.0, 1.0) * t;
        let x = ((1.0 - t) * (self.width - 1) as f32).round() as usize;
        let y = ((1.0 - d) * (self.height - 1) as f32).round() as usize;
        self.rgb[y * self.width as usize + x].map(|c| f32::from(c) / 255.0)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BiomePalette {
    pub grass: [[f32; 3]; BIOME_COUNT],
    pub foliage: [[f32; 3]; BIOME_COUNT],
}

impl BiomePalette {
    pub fn constants() -> BiomePalette {
        BiomePalette {
            grass: GRASS.map(rgb),
            foliage: FOLIAGE.map(rgb),
        }
    }

    pub fn from_colormaps(grass: Option<&Colormap>, foliage: Option<&Colormap>) -> BiomePalette {
        let constants = BiomePalette::constants();
        let sample = |map: Option<&Colormap>, fallback: [[f32; 3]; BIOME_COUNT]| {
            let mut colors = fallback;
            if let Some(map) = map {
                for (color, (temperature, downfall)) in colors.iter_mut().zip(CLIMATE) {
                    *color = map.at(temperature, downfall);
                }
            }
            colors
        };
        BiomePalette {
            grass: sample(grass, constants.grass),
            foliage: sample(foliage, constants.foliage),
        }
    }

    pub fn colors(&self, kind: BiomeTint) -> Option<&[[f32; 3]; BIOME_COUNT]> {
        match kind {
            BiomeTint::None => None,
            BiomeTint::Grass => Some(&self.grass),
            BiomeTint::Foliage => Some(&self.foliage),
        }
    }

    pub fn plains(&self, kind: BiomeTint) -> [f32; 3] {
        self.colors(kind).map_or([1.0; 3], |colors| colors[0])
    }

    /// Раскладка `palette` в `shader.wgsl`: восемь `vec4`, сначала трава, потом листва.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.grass
            .iter()
            .chain(&self.foliage)
            .flat_map(|c| [c[0], c[1], c[2], 0.0])
            .flat_map(f32::to_le_bytes)
            .collect()
    }
}

pub fn weights_bytes(w: BiomeWeights) -> [u8; BIOME_COUNT] {
    [w.plains, w.desert, w.forest, w.taiga].map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
}

pub fn blend(colors: &[[f32; 3]; BIOME_COUNT], weights: [u8; BIOME_COUNT]) -> [f32; 3] {
    let sum: f32 = weights.iter().map(|&w| f32::from(w)).sum();
    if sum == 0.0 {
        return colors[0];
    }
    let mut out = [0.0; 3];
    for (color, &w) in colors.iter().zip(&weights) {
        for c in 0..3 {
            out[c] += color[c] * f32::from(w) / sum;
        }
    }
    out
}

#[cfg(test)]
mod tests;
