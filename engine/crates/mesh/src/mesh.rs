use std::sync::LazyLock;

use world::light::{MAX_LIGHT, level_brightness};

pub const POSITION_UNITS: i32 = 16;

const LIGHT_MASK: u32 = 0x1F;

pub const MAX_LIGHT_HALVES: u8 = 2 * MAX_LIGHT;

pub fn light_brightness(sun_halves: u8, block_halves: u8, darkness: f32) -> f32 {
    let sun = f32::from(sun_halves) / 2.0 - darkness;
    level_brightness(sun.max(f32::from(block_halves) / 2.0))
}

pub fn day_shade(shade: u8, sun_halves: u8, block_halves: u8) -> u8 {
    static TABLE: LazyLock<[f32; MAX_LIGHT_HALVES as usize + 1]> =
        LazyLock::new(|| std::array::from_fn(|h| level_brightness(h as f32 / 2.0)));
    let light = TABLE[usize::from(sun_halves.max(block_halves))];
    quantize_shade(f32::from(shade) / 255.0 * light)
}

pub fn quantize_shade(shade: f32) -> u8 {
    (shade.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Раскладку знает и `shader.wgsl` (`vs_main`): менять вместе.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quad([u32; 3]);

pub const QUAD_BYTES: usize = size_of::<Quad>();

const _: () = assert!(QUAD_BYTES == 12);
const _: () = assert!(crate::templates::TEMPLATE_COUNT <= 64);

impl Quad {
    pub fn new(
        cell: [u8; 3],
        template: u8,
        flip: bool,
        layer: u8,
        corner_ao: [u8; 4],
        sun: [u8; 4],
        block: [u8; 4],
    ) -> Self {
        debug_assert!(cell.iter().all(|&c| c < 16), "{cell:?}");
        debug_assert!(usize::from(template) < crate::templates::TEMPLATE_COUNT);
        debug_assert!(corner_ao.iter().all(|&a| a < 4), "{corner_ao:?}");
        debug_assert!(sun.iter().chain(&block).all(|&l| l <= MAX_LIGHT_HALVES));
        let pack = |values: [u8; 4], bits: u32| {
            values
                .iter()
                .enumerate()
                .fold(0, |acc, (i, &v)| acc | u32::from(v) << (bits * i as u32))
        };
        let [x, y, z] = cell.map(u32::from);
        Self([
            x | y << 4
                | z << 8
                | u32::from(template & 0x1F) << 12
                | u32::from(flip) << 17
                | u32::from(layer) << 18
                | u32::from(template >> 5) << 26,
            pack(corner_ao, 2) | pack(sun, 5) << 8,
            pack(block, 5),
        ])
    }

    pub fn words(self) -> [u32; 3] {
        self.0
    }

    pub fn from_words(words: [u32; 3]) -> Self {
        Self(words)
    }

    pub fn cell(self) -> [u8; 3] {
        let a = self.0[0];
        [a, a >> 4, a >> 8].map(|v| (v & 0xF) as u8)
    }

    pub fn template(self) -> u8 {
        ((self.0[0] >> 12 & 0x1F) | (self.0[0] >> 26 & 1) << 5) as u8
    }

    pub fn flipped(self) -> bool {
        self.0[0] >> 17 & 1 != 0
    }

    pub fn layer(self) -> u8 {
        (self.0[0] >> 18) as u8
    }

    pub fn ao(self) -> [u8; 4] {
        std::array::from_fn(|i| (self.0[1] >> (2 * i) & 3) as u8)
    }

    pub fn sun_halves(self) -> [u8; 4] {
        std::array::from_fn(|i| (self.0[1] >> (8 + 5 * i) & LIGHT_MASK) as u8)
    }

    pub fn block_halves(self) -> [u8; 4] {
        std::array::from_fn(|i| (self.0[2] >> (5 * i) & LIGHT_MASK) as u8)
    }
}

pub fn quad_bytes(quads: &[Quad]) -> Vec<u8> {
    quads
        .iter()
        .flat_map(|q| q.0)
        .flat_map(u32::to_le_bytes)
        .collect()
}

pub const SLOT_BYTES: usize = 16;

pub fn slot_bytes(origin: [f32; 3], first_word: u32) -> [u8; SLOT_BYTES] {
    let mut bytes = [0; SLOT_BYTES];
    for (chunk, f) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(origin) {
        chunk.copy_from_slice(&f.to_le_bytes());
    }
    bytes[12..].copy_from_slice(&first_word.to_le_bytes());
    bytes
}

#[cfg(test)]
mod tests;
