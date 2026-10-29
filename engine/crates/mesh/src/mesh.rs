use std::sync::LazyLock;

use world::light::{MAX_LIGHT, level_brightness};

/// Шагов позиции на блок: позиция вершины — целое число шестнадцатых долей блока.
/// Хватает и для воды (верх на 14/16 = 0.875), и для любой части блока в будущем.
pub const POSITION_UNITS: i32 = 16;

const LIGHT_MASK: u32 = 0x1F;

/// Наибольшее значение поля света вершины: 15 уровней в половинках.
pub const MAX_LIGHT_HALVES: u8 = 2 * MAX_LIGHT;

/// Яркость света вершины: `brightness(max(солнце − тьма, блоки))` по уровням, которые
/// хранятся в половинках. Шейдер (`vertex_brightness`) считает так же.
pub fn light_brightness(sun_halves: u8, block_halves: u8, darkness: f32) -> f32 {
    let sun = f32::from(sun_halves) / 2.0 - darkness;
    level_brightness(sun.max(f32::from(block_halves) / 2.0))
}

/// Дневная яркость вершины (тьма 0) в байте: `shade` × яркость большего из двух
/// уровней. Мешер выбирает по ней диагональ квада — по тем же числам, что днём увидит
/// шейдер. Яркости уровней в таблице: `powf` на каждую вершину заметно удорожал бы меш.
pub fn day_shade(shade: u8, sun_halves: u8, block_halves: u8) -> u8 {
    static TABLE: LazyLock<[f32; MAX_LIGHT_HALVES as usize + 1]> =
        LazyLock::new(|| std::array::from_fn(|h| level_brightness(h as f32 / 2.0)));
    let light = TABLE[usize::from(sun_halves.max(block_halves))];
    quantize_shade(f32::from(shade) / 255.0 * light)
}

/// Яркость 0..=1 → байт вершины. Мешер сравнивает яркости уже по этим байтам, чтобы
/// диагональ квада выбиралась по тем же числам, что увидит шейдер.
pub fn quantize_shade(shade: f32) -> u8 {
    (shade.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Запись квада, 12 байт: три `u32`. Квад — одна грань (куб, вода, плоскость креста или
/// бок факела); четыре вершины шейдер достраивает по номеру шаблона
/// ([`crate::templates`]), поэтому в записи только то, что меняется от грани к грани.
/// Позиция локальная (от угла секции), мировой угол секции приходит отдельно, вместе с
/// началом записей секции в арене ([`slot_bytes`]).
///
/// Раскладка (младший бит первым):
/// - слово 0: блок в секции x 0..4, y 4..8, z 8..12; шаблон 12..17 (5 бит, 25 шаблонов);
///   переворот диагонали 17; слой текстуры 18..26 (8 бит); 26..32 свободны;
/// - слово 1: AO четырёх углов 0..8 (по 2 бита, 0..=3, яркость — [`crate::templates::AO_BRIGHTNESS`]);
///   свет солнца углов 8..28 (по 5 бит в половинках уровня, 0..=30, [`MAX_LIGHT_HALVES`]);
///   28..32 свободны;
/// - слово 2: свет блоков углов 0..20 (так же); 20..32 свободны.
///
/// Углы — в порядке углов шаблона. Диагональ квада: без переворота треугольники
/// `0,1,2` и `0,2,3`, с переворотом — `1,2,3` и `1,3,0`; мешер выбирает её против
/// анизотропии AO ([`mesher`](crate::mesher)). Яркость стороны хранит шаблон, яркость
/// от света шейдер считает из уровней и тьмы неба (`Globals`): ночью меняется только
/// uniform. Половинки уровня дают ошибку яркости не больше 0.8^0.25 ≈ 5.5%: «средний»
/// уровень вершины — дробь со знаменателем 1..4, а бит на точность дороже, чем на неё
/// хватает глазу.
///
/// Раскладку знает и `shader.wgsl` (`vs_main`): менять вместе.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quad([u32; 3]);

/// Размер записи квада, байт.
pub const QUAD_BYTES: usize = size_of::<Quad>();

const _: () = assert!(QUAD_BYTES == 12);

impl Quad {
    /// `cell` — блок в секции (0..16 по каждой оси), `corner_ao` — уровни AO углов,
    /// `sun` и `block` — свет углов в половинках уровня.
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
                | u32::from(template) << 12
                | u32::from(flip) << 17
                | u32::from(layer) << 18,
            pack(corner_ao, 2) | pack(sun, 5) << 8,
            pack(block, 5),
        ])
    }

    /// Три слова записи: так квад пересекает границу воркера ([`crate::job`]).
    pub fn words(self) -> [u32; 3] {
        self.0
    }

    /// Обратное к [`Quad::words`]; слова должны быть из `words`, не проверяются.
    pub fn from_words(words: [u32; 3]) -> Self {
        Self(words)
    }

    pub fn cell(self) -> [u8; 3] {
        let a = self.0[0];
        [a, a >> 4, a >> 8].map(|v| (v & 0xF) as u8)
    }

    pub fn template(self) -> u8 {
        (self.0[0] >> 12 & 0x1F) as u8
    }

    /// Диагональ перевёрнута: треугольники `1,2,3` и `1,3,0`.
    pub fn flipped(self) -> bool {
        self.0[0] >> 17 & 1 != 0
    }

    pub fn layer(self) -> u8 {
        (self.0[0] >> 18) as u8
    }

    pub fn ao(self) -> [u8; 4] {
        std::array::from_fn(|i| (self.0[1] >> (2 * i) & 3) as u8)
    }

    /// Свет солнца четырёх углов в половинках уровня.
    pub fn sun_halves(self) -> [u8; 4] {
        std::array::from_fn(|i| (self.0[1] >> (8 + 5 * i) & LIGHT_MASK) as u8)
    }

    /// Свет блоков четырёх углов в половинках уровня.
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

/// Данные секции для отрисовки одного меша, байт.
pub const SLOT_BYTES: usize = 16;

/// Слот меша (`first_instance` отрисовки): мировой угол секции тремя `f32` и начало
/// записей квадов меша в арене, в словах `u32` (смещение в байтах / 4). Четыре байта
/// после угла у `vec3<f32>` в шейдере всё равно пустовали.
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
