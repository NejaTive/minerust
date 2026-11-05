use std::sync::LazyLock;

use world::light::{MAX_LIGHT, level_brightness};

pub const POSITION_UNITS: i32 = 16;

const LIGHT_MASK: u32 = 0x1F;
const BOX_FLAG: u32 = 1 << 29;
const BOX_AO_SHIFT: u32 = 13;
const EXT_FLAG: u32 = 1 << 31;
const EXT_SHIFT: u32 = 28;

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
const _: () = assert!(crate::templates::TEMPLATE_COUNT <= 2048);

impl Quad {
    pub fn new(
        cell: [u8; 3],
        template: u8,
        flip: bool,
        layer: u16,
        corner_ao: [u8; 4],
        sun: [u8; 4],
        block: [u8; 4],
    ) -> Self {
        debug_assert!(cell.iter().all(|&c| c < 16), "{cell:?}");
        debug_assert!(usize::from(template) < crate::templates::TEMPLATE_COUNT.min(256));
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
                | u32::from(layer & 0xFF) << 18
                | u32::from(template >> 5) << 26
                | u32::from(layer >> 8) << 30,
            pack(corner_ao, 2) | pack(sun, 5) << 8,
            pack(block, 5),
        ])
    }

    /// Грань коробки произвольного размера: `face` — индекс грани `FACES` (она же шаблон),
    /// `rect` — `(u0, u1, v0, v1)` в 1/16 на осях грани (`world::boxes::face_axes`), `plane` —
    /// положение грани по нормали, `light` — `[sun, block]` на всю грань, `ao` — по углам шаблона.
    /// Биты AO и высот заняты прямоугольником и одним светом; раскладку знает и shader.wgsl.
    pub fn boxed(
        cell: [u8; 3],
        face: u8,
        rect: (u8, u8, u8, u8),
        plane: u8,
        layer: u16,
        light: [u8; 2],
        ao: [u8; 4],
    ) -> Self {
        let (u0, u1, v0, v1) = rect;
        debug_assert!(face < 6 && u0 < u1 && v0 < v1 && u1 <= 16 && v1 <= 16 && plane <= 16);
        debug_assert!(ao.iter().all(|&a| a < 4), "{ao:?}");
        let mut quad = Self::new(
            cell,
            face,
            false,
            layer,
            [0; 4],
            [light[0], 0, 0, 0],
            [light[1], 0, 0, 0],
        );
        let code = if face.is_multiple_of(2) {
            plane - 1
        } else {
            plane
        };
        quad.0[0] |= BOX_FLAG;
        let ao = ao
            .iter()
            .enumerate()
            .fold(0, |acc, (i, &a)| acc | u32::from(a) << (2 * i));
        quad.0[1] |=
            u32::from(u0) | u32::from(u1 - 1) << 4 | ao << BOX_AO_SHIFT | u32::from(code) << 28;
        quad.0[2] |= u32::from(v0) << 20 | u32::from(v1 - 1) << 24;
        quad
    }

    /// Квад шаблона с номером до 2047: старшие биты номера лежат в `w2` вместе с флагом (его читает `vs_main`),
    /// свет и AO — как у коробочной грани: один свет на квад, AO по углам.
    pub fn extended(cell: [u8; 3], template: u16, layer: u16, light: [u8; 2], ao: [u8; 4]) -> Self {
        debug_assert!(usize::from(template) < crate::templates::TEMPLATE_COUNT);
        debug_assert!(ao.iter().all(|&a| a < 4), "{ao:?}");
        let mut quad = Self::new(
            cell,
            (template & 0xFF) as u8,
            false,
            layer,
            [0; 4],
            [light[0], 0, 0, 0],
            [light[1], 0, 0, 0],
        );
        quad.0[0] |= BOX_FLAG;
        let ao = ao
            .iter()
            .enumerate()
            .fold(0, |acc, (i, &a)| acc | u32::from(a) << (2 * i));
        quad.0[1] |= ao << BOX_AO_SHIFT;
        quad.0[2] |= EXT_FLAG | u32::from(template >> 8) << EXT_SHIFT;
        quad
    }

    pub fn is_extended(self) -> bool {
        self.0[0] & BOX_FLAG != 0 && self.0[2] & EXT_FLAG != 0
    }

    /// Поворот текстуры коробочной грани на `turns` четвертей по часовой стрелке сверху; в записи `w2` биты 28–29.
    pub fn with_turn(mut self, turns: u8) -> Self {
        debug_assert!(self.is_boxed() && turns < 4);
        self.0[2] |= u32::from(turns) << 28;
        self
    }

    pub fn turn(self) -> u8 {
        (self.0[2] >> 28 & 3) as u8
    }

    pub fn is_boxed(self) -> bool {
        self.0[0] & BOX_FLAG != 0 && !self.is_extended()
    }

    fn single_light(self) -> bool {
        self.0[0] & BOX_FLAG != 0
    }

    /// `(rect, plane)` коробочной грани, см. [`Quad::boxed`].
    pub fn box_rect(self) -> ((u8, u8, u8, u8), u8) {
        let field = |word: usize, shift: u32| (self.0[word] >> shift & 0xF) as u8;
        let code = field(1, 28);
        let plane = if self.template().is_multiple_of(2) {
            code + 1
        } else {
            code
        };
        (
            (field(1, 0), field(1, 4) + 1, field(2, 20), field(2, 24) + 1),
            plane,
        )
    }

    // шаги 1/16 блока; биты полей знает и shader.wgsl (vs_main)
    pub fn with_heights(mut self, heights: [u8; 4]) -> Self {
        debug_assert!(heights.iter().all(|h| (1..=16).contains(h)), "{heights:?}");
        let [h0, h1, h2, h3] = heights.map(|h| u32::from(h - 1));
        self.0[1] |= h0 << 28;
        self.0[2] |= h1 << 20 | h2 << 24 | h3 << 28;
        self
    }

    pub fn heights(self) -> [u8; 4] {
        [
            (self.0[1] >> 28 & 0xF) as u8 + 1,
            (self.0[2] >> 20 & 0xF) as u8 + 1,
            (self.0[2] >> 24 & 0xF) as u8 + 1,
            (self.0[2] >> 28 & 0xF) as u8 + 1,
        ]
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

    /// Младшие 8 бит номера шаблона: у расширенных квадов полный номер — [`Quad::template_index`].
    pub fn template(self) -> u8 {
        ((self.0[0] >> 12 & 0x1F) | (self.0[0] >> 26 & 7) << 5) as u8
    }

    pub fn template_index(self) -> u16 {
        let high = if self.is_extended() {
            self.0[2] >> EXT_SHIFT & 7
        } else {
            0
        };
        u16::from(self.template()) | (high as u16) << 8
    }

    pub fn flipped(self) -> bool {
        self.0[0] >> 17 & 1 != 0
    }

    pub fn layer(self) -> u16 {
        ((self.0[0] >> 18 & 0xFF) | (self.0[0] >> 30) << 8) as u16
    }

    pub fn ao(self) -> [u8; 4] {
        if self.single_light() {
            return std::array::from_fn(|i| (self.0[1] >> (BOX_AO_SHIFT + 2 * i as u32) & 3) as u8);
        }
        std::array::from_fn(|i| (self.0[1] >> (2 * i) & 3) as u8)
    }

    pub fn sun_halves(self) -> [u8; 4] {
        if self.single_light() {
            return [(self.0[1] >> 8 & LIGHT_MASK) as u8; 4];
        }
        std::array::from_fn(|i| (self.0[1] >> (8 + 5 * i) & LIGHT_MASK) as u8)
    }

    pub fn block_halves(self) -> [u8; 4] {
        if self.single_light() {
            return [(self.0[2] & LIGHT_MASK) as u8; 4];
        }
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
