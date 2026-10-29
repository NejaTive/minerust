//! Шаблоны квадов: геометрия, которую шейдер достраивает по номеру шаблона в записи
//! [`Quad`](crate::mesh::Quad). Одна таблица на Rust и WGSL: шейдер получает её байтами
//! из [`table_bytes`] (uniform-буфер рендера), так что расхождение невозможно; размер
//! структуры в шейдере сверяет тест рендера.
//!
//! Шаблон — четыре угла (положение в 1/16 блока от угла блока и UV в 1/16 тайла) и яркость
//! стороны. Углы идут против часовой стрелки при взгляде снаружи (`FrontFace::Ccw`,
//! `cull_mode: Back`); яркость угла = `shade` × [`AO_BRIGHTNESS`] по AO из записи.

use std::sync::LazyLock;

use glam::IVec3;

use crate::mesh::POSITION_UNITS;
use world::block::{Facing, WATER_SURFACE};
use world::tile::{TILE_SIZE, TORCH_PIXELS, TORCH_TOP_ROW};

pub(crate) struct Face {
    /// Направление на соседа, которого грань касается.
    pub(crate) normal: IVec3,
    pub(crate) facing: Facing,
    /// Множитель яркости, как в Minecraft без сглаженного света.
    pub(crate) shade: f32,
    /// Углы в единичном кубе блока, против часовой стрелки при взгляде снаружи. У боковых
    /// граней порядок всегда низ-лево, низ-право, верх-право, верх-лево — на этом держится
    /// `CORNER_UVS`.
    pub(crate) corners: [[i32; 3]; 4],
}

/// UV углов грани в порядке `Face::corners`: у боковых граней v = 0 оказывается
/// сверху, и кайма травы смотрит вверх.
pub(crate) const CORNER_UVS: [[u8; 2]; 4] = [[0, 16], [16, 16], [16, 0], [0, 0]];

pub(crate) const FACES: [Face; 6] = [
    Face {
        normal: IVec3::Y,
        facing: Facing::Top,
        shade: 1.0,
        corners: [[0, 1, 1], [1, 1, 1], [1, 1, 0], [0, 1, 0]],
    },
    Face {
        normal: IVec3::NEG_Y,
        facing: Facing::Bottom,
        shade: 0.5,
        corners: [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]],
    },
    Face {
        normal: IVec3::X,
        facing: Facing::Side,
        shade: 0.6,
        corners: [[1, 0, 1], [1, 0, 0], [1, 1, 0], [1, 1, 1]],
    },
    Face {
        normal: IVec3::NEG_X,
        facing: Facing::Side,
        shade: 0.6,
        corners: [[0, 0, 0], [0, 0, 1], [0, 1, 1], [0, 1, 0]],
    },
    Face {
        normal: IVec3::Z,
        facing: Facing::Side,
        shade: 0.8,
        corners: [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]],
    },
    Face {
        normal: IVec3::NEG_Z,
        facing: Facing::Side,
        shade: 0.8,
        corners: [[1, 0, 0], [0, 0, 0], [0, 1, 0], [1, 1, 0]],
    },
];

/// Верх воды в шагах позиции вершины ([`WATER_SURFACE`] × 16; точно, потому что
/// 0.875 = 14/16).
pub(crate) const WATER_SURFACE_UNITS: i32 = 14;
const _: () = assert!(WATER_SURFACE_UNITS as f32 == WATER_SURFACE * POSITION_UNITS as f32);

/// Яркость сторон креста: свет не зависит от направления, растение не должно быть
/// темнее одной стороной.
pub(crate) const CROSS_SHADE: f32 = 1.0;

/// Столбик факела в шагах позиции: центр клетки, [`TORCH_PIXELS`] по x и z, высота
/// от низа клетки до верха тайла без `TORCH_TOP_ROW` строк (10 из 16).
pub(crate) const TORCH_BOX: ([i32; 3], [i32; 3]) = (
    [TORCH_PIXELS.0 as i32, 0, TORCH_PIXELS.0 as i32],
    [
        TORCH_PIXELS.1 as i32,
        (TILE_SIZE - TORCH_TOP_ROW) as i32,
        TORCH_PIXELS.1 as i32,
    ],
);

/// Яркость вершины по уровню AO (`vertex_ao`). Не 0.25-шаг по 0fps: вместе с `shade`
/// нижней грани (0.5) угол стал бы почти чёрным и съел текстуру. Нижняя ступень
/// темнее остальных (0.2 против 0.15), чтобы внутренний угол из двух стен читался
/// отдельно от основания одной стены. Шейдер получает массив из [`table_bytes`].
pub const AO_BRIGHTNESS: [f32; 4] = [0.5, 0.7, 0.85, 1.0];

// Номера шаблонов. Куб: `CUBE_FACE` + номер грани в `FACES`; остальные группы — смещение
// внутри группы, как описано у константы.
/// Шесть граней куба в порядке `FACES`.
pub const CUBE_FACE: u8 = 0;
/// Верх воды, опущенный до [`WATER_SURFACE_UNITS`]. Верх на полной высоте не нужен:
/// над такой водой всегда вода, и грани нет.
pub const WATER_TOP: u8 = 6;
/// Четыре боковые грани воды с полным верхом (над ней вода), порядок — боковых `FACES`.
pub const WATER_SIDE: u8 = 7;
/// То же с опущенным верхом: UV обрезана (v верха = 2), а не сжата.
pub const WATER_SIDE_LOW: u8 = 11;
pub const WATER_BOTTOM: u8 = 15;
/// Креста: `2 · плоскость + обратная сторона` (обратная — те же углы в обратном порядке,
/// иначе `cull_mode: Back` скроет её).
pub const CROSS: u8 = 16;
/// Четыре бока факела, порядок — боковых `FACES`.
pub const TORCH_SIDE: u8 = 20;
pub const TORCH_TOP: u8 = 24;
pub const TEMPLATE_COUNT: usize = 25;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corner {
    /// Шаги 1/16 блока от угла блока, 0..=16.
    pub pos: [u8; 3],
    /// Шаги 1/16 тайла, 0..=16; v = 0 — верх тайла.
    pub uv: [u8; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Template {
    pub corners: [Corner; 4],
    /// Яркость стороны до AO, 0..=1.
    pub shade: f32,
}

/// Все шаблоны; индекс — номер шаблона в записи квада.
pub fn templates() -> &'static [Template; TEMPLATE_COUNT] {
    static TEMPLATES: LazyLock<[Template; TEMPLATE_COUNT]> = LazyLock::new(build);
    &TEMPLATES
}

fn cube_face(face: &Face, top: i32) -> Template {
    // `top` — высота, в которую приходит единица по y: 16 у куба, 14 у воды с опущенным
    // верхом. На боковой грани v = 1 − y, как у полной: опущенный верх обрезает тайл.
    let corners = std::array::from_fn(|i| {
        let [x, y, z] = face.corners[i];
        let height = if y == 1 { top } else { 0 };
        let [u, mut v] = CORNER_UVS[i];
        if face.facing == Facing::Side {
            v = (POSITION_UNITS - height) as u8;
        }
        Corner {
            pos: [
                (x * POSITION_UNITS) as u8,
                height as u8,
                (z * POSITION_UNITS) as u8,
            ],
            uv: [u, v],
        }
    });
    Template {
        corners,
        shade: face.shade,
    }
}

fn torch_face(face: &Face) -> Template {
    let (lo, hi) = TORCH_BOX;
    let corners = face.corners.map(|[cx, cy, cz]| {
        let c = [cx, cy, cz];
        let p: [i32; 3] = std::array::from_fn(|i| if c[i] == 1 { hi[i] } else { lo[i] });
        let span = (TORCH_PIXELS.1 - TORCH_PIXELS.0) as i32;
        // Текстура берётся по центру тайла: боковая грань — древко с огоньком
        // сверху (v от верха столбика), верхняя — два верхних ряда огонька.
        let uv = if face.facing == Facing::Top {
            [
                TORCH_PIXELS.0 as i32 + span * cx,
                TORCH_TOP_ROW as i32 + 2 * (1 - cz),
            ]
        } else {
            let side = if face.normal.x != 0 { cz } else { cx };
            let side = if face.normal.x > 0 || face.normal.z < 0 {
                1 - side
            } else {
                side
            };
            [
                TORCH_PIXELS.0 as i32 + span * side,
                TORCH_TOP_ROW as i32 + (TILE_SIZE as i32 - TORCH_TOP_ROW as i32) * (1 - cy),
            ]
        };
        Corner {
            pos: p.map(|v| v as u8),
            uv: uv.map(|v| v as u8),
        }
    });
    Template {
        corners,
        shade: face.shade,
    }
}

fn build() -> [Template; TEMPLATE_COUNT] {
    let mut out = Vec::with_capacity(TEMPLATE_COUNT);
    let sides = &FACES[2..];
    out.extend(FACES.iter().map(|f| cube_face(f, POSITION_UNITS)));
    out.push(cube_face(&FACES[0], WATER_SURFACE_UNITS));
    out.extend(sides.iter().map(|f| cube_face(f, POSITION_UNITS)));
    out.extend(sides.iter().map(|f| cube_face(f, WATER_SURFACE_UNITS)));
    out.push(cube_face(&FACES[1], WATER_SURFACE_UNITS));
    let u = POSITION_UNITS as u8;
    // Две диагонали клетки; каждая — квад в обе стороны.
    for (a, b) in [([0, 0], [u, u]), ([u, 0], [0, u])] {
        let corner = |p: [u8; 2], y: u8, uv: [u8; 2]| Corner {
            pos: [p[0], y, p[1]],
            uv,
        };
        let front = [
            corner(a, 0, [0, u]),
            corner(b, 0, [u, u]),
            corner(b, u, [u, 0]),
            corner(a, u, [0, 0]),
        ];
        let mut back = front;
        back.reverse();
        for corners in [front, back] {
            out.push(Template {
                corners,
                shade: CROSS_SHADE,
            });
        }
    }
    out.extend(sides.iter().map(torch_face));
    out.push(torch_face(&FACES[0]));
    out.try_into().expect("template count")
}

/// Угол шаблона в одном `u32`: x, y, z, u, v по 5 бит (0..=16), младшие первыми. Так
/// же распаковывает шейдер.
pub fn pack_corner(c: Corner) -> u32 {
    let [x, y, z] = c.pos.map(u32::from);
    let [u, v] = c.uv.map(u32::from);
    x | y << 5 | z << 10 | u << 15 | v << 20
}

/// Размер [`table_bytes`], байт: шаблоны по 32 байта (углы `vec4<u32>` и яркость
/// `vec4<f32>` с пустыми компонентами: у uniform-массива шаг кратен 16) и четыре
/// яркости AO.
pub const TABLE_BYTES: usize = TEMPLATE_COUNT * 32 + 16;

/// Таблица шаблонов для шейдера (`ShapeTable` в `shader.wgsl`, uniform-буфер): на шаблон
/// четыре упакованных угла ([`pack_corner`]) и яркость стороны, затем [`AO_BRIGHTNESS`].
/// Таблица в uniform, а не в `const` шейдера: динамический индекс по `const`-массиву
/// компилятор копирует в локальный массив на каждую вершину.
pub fn table_bytes() -> Vec<u8> {
    let mut words: Vec<u32> = Vec::with_capacity(TABLE_BYTES / 4);
    for t in templates() {
        words.extend(t.corners.map(pack_corner));
        words.extend([t.shade.to_bits(), 0, 0, 0]);
    }
    words.extend(AO_BRIGHTNESS.map(f32::to_bits));
    words.into_iter().flat_map(u32::to_le_bytes).collect()
}
