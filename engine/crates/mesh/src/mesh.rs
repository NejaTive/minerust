use std::sync::LazyLock;

use world::light::{MAX_LIGHT, level_brightness};

/// Шагов позиции на блок: позиция вершины — целое число шестнадцатых долей блока.
/// Хватает и для воды (верх на 14/16 = 0.875), и для любой части блока в будущем.
pub const POSITION_UNITS: i32 = 16;

/// Наибольшее значение координаты вершины в шагах: 16 блоков секции × 16 шагов.
pub const MAX_POSITION: u16 = 16 * POSITION_UNITS as u16;

/// Вершина секции, 8 байт: два `u32`. Позиция локальная (от угла секции), мировой
/// угол секции приходит отдельным буфером на отрисовку ([`origin_layout`]).
///
/// Раскладка (младший бит первым):
/// - слово 0: x 0..9, y 9..18, z 18..27 (по 9 бит, 0..=256 шагов по 1/16 блока, то
///   есть 0..=16 блоков включительно), u 27..32 (5 бит, 0..=16 шестнадцатых тайла);
/// - слово 1: v 0..5 (так же), слой текстуры 5..13 (8 бит), яркость 13..21
///   (8 бит, 0..=255 — `/255` в шейдере);
/// - слово 1: свет солнца 21..26 и свет блоков 26..31 — средний по вершине уровень в
///   половинках уровня (5 бит, 0..=30, [`MAX_LIGHT_HALVES`]); бит 31 свободен.
///
/// Яркость от света шейдер считает сам из этих двух уровней и тьмы неба (`Globals`):
/// ночью меняется только uniform, меши остаются теми же. Половинки уровня дают
/// ошибку яркости не больше 0.8^0.25 ≈ 5.5%: «средний» уровень вершины — дробь со
/// знаменателем 1..4, а бит на точность дороже, чем на неё хватает глазу.
///
/// UV хранятся явно, а не выводятся из номера угла: у боковых граней воды под опущенным
/// верхом v = 1 − y (обрезка тайла), и угла грани для этого мало.
///
/// Раскладку знает и `shader.wgsl` (`vs_main`): менять вместе.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vertex([u32; 2]);

const POSITION_MASK: u32 = 0x1FF;
const UV_MASK: u32 = 0x1F;
const LIGHT_MASK: u32 = 0x1F;

/// Наибольшее значение поля света вершины: 15 уровней в половинках.
pub const MAX_LIGHT_HALVES: u8 = 2 * MAX_LIGHT;

impl Vertex {
    /// `position` — шаги 1/16 блока от угла секции (0..=[`MAX_POSITION`]), `uv` — шаги
    /// 1/16 тайла (0..=16, v = 0 — верх тайла), `shade` — сторона × AO, 0..=255,
    /// `light` — `[солнце, блоки]` в половинках уровня (0..=[`MAX_LIGHT_HALVES`]).
    pub fn new(position: [u16; 3], uv: [u8; 2], shade: u8, layer: u8, light: [u8; 2]) -> Self {
        debug_assert!(light.iter().all(|&l| l <= MAX_LIGHT_HALVES), "{light:?}");
        debug_assert!(position.iter().all(|&p| p <= MAX_POSITION), "{position:?}");
        debug_assert!(uv.iter().all(|&c| i32::from(c) <= POSITION_UNITS), "{uv:?}");
        let [x, y, z] = position.map(u32::from);
        Self([
            x | y << 9 | z << 18 | u32::from(uv[0]) << 27,
            u32::from(uv[1])
                | u32::from(layer) << 5
                | u32::from(shade) << 13
                | u32::from(light[0]) << 21
                | u32::from(light[1]) << 26,
        ])
    }

    /// Позиция в блоках от угла секции.
    pub fn position(self) -> [f32; 3] {
        let [a, _] = self.0;
        [a, a >> 9, a >> 18].map(|w| (w & POSITION_MASK) as f32 / POSITION_UNITS as f32)
    }

    /// UV в долях тайла.
    pub fn uv(self) -> [f32; 2] {
        let [a, b] = self.0;
        [a >> 27, b & UV_MASK].map(|c| c as f32 / POSITION_UNITS as f32)
    }

    /// Яркость 0..=1 (по квантованному значению).
    pub fn shade(self) -> f32 {
        f32::from(self.shade_byte()) / 255.0
    }

    pub fn shade_byte(self) -> u8 {
        (self.0[1] >> 13) as u8
    }

    /// Свет солнца в половинках уровня.
    pub fn sun_halves(self) -> u8 {
        (self.0[1] >> 21 & LIGHT_MASK) as u8
    }

    /// Свет блоков в половинках уровня.
    pub fn block_halves(self) -> u8 {
        (self.0[1] >> 26 & LIGHT_MASK) as u8
    }

    /// Итоговая яркость 0..=1 при тьме неба `darkness` (в уровнях): то же, что считает
    /// шейдер. Нужна тестам и выбору диагонали квада.
    pub fn brightness(self, darkness: f32) -> f32 {
        self.shade() * light_brightness(self.sun_halves(), self.block_halves(), darkness)
    }

    pub fn layer(self) -> u8 {
        (self.0[1] >> 5) as u8
    }
}

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

/// Размер буфера угла секции, байт.
pub const ORIGIN_BYTES: usize = 16;

pub fn origin_bytes(origin: [f32; 3]) -> [u8; ORIGIN_BYTES] {
    let mut bytes = [0; ORIGIN_BYTES];
    for (chunk, f) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(origin) {
        chunk.copy_from_slice(&f.to_le_bytes());
    }
    bytes
}

pub fn vertex_bytes(vertices: &[Vertex]) -> Vec<u8> {
    vertices
        .iter()
        .flat_map(|v| v.0)
        .flat_map(u32::to_le_bytes)
        .collect()
}

/// Квадов в самой большой секции: вершины нумеруются `u16`, 65536 вершин / 4.
pub const MAX_QUADS: usize = 1 << 14;

/// Индексы `0,1,2, 0,2,3` для каждого из [`MAX_QUADS`] квадов подряд: один буфер на все
/// секции вместо своих индексов у каждой (минус 3 байта на вершину).
pub fn quad_index_bytes() -> Vec<u8> {
    (0..MAX_QUADS)
        .flat_map(|q| {
            let first = (q * 4) as u16;
            [0, 1, 2, 0, 2, 3].map(|i| first + i)
        })
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// Вершины секции в порядке для общих индексов `0,1,2, 0,2,3`: у квада, который
/// мешер разрезал по другой диагонали (`1,2,3, 1,3,0`), вершины сдвигаются на одну,
/// и те же индексы дают те же треугольники с тем же обходом.
pub fn gpu_vertices(vertices: &[Vertex], indices: &[u16]) -> Vec<Vertex> {
    let mut out = Vec::with_capacity(vertices.len());
    for (q, quad) in vertices.as_chunks::<4>().0.iter().enumerate() {
        let first = (q * 4) as u16;
        match indices[q * 6] - first {
            0 => out.extend_from_slice(quad),
            1 => out.extend([quad[1], quad[2], quad[3], quad[0]]),
            other => unreachable!("quad {q} starts at vertex {other}"),
        }
    }
    out
}

#[cfg(test)]
mod tests;
