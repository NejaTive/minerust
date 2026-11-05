//! [`Quad`](crate::mesh::Quad). Одна таблица на Rust и WGSL: шейдер получает её байтами
//! из [`table_bytes`] (uniform-буфер рендера), так что расхождение невозможно; размер
//! структуры в шейдере сверяет тест рендера.

use std::sync::LazyLock;

use glam::IVec3;

use crate::mesh::POSITION_UNITS;
use world::block::{
    CACTUS_INSET_UNITS, COCOA_BACK, COCOA_PODS, COCOA_STAGES, COCOA_STEM, COCOA_STEM_DEPTH, Facing,
    LILY_UNITS, RAIL_ASCENDING_EAST, RAIL_ASCENDING_NORTH, RAIL_ASCENDING_SOUTH,
    RAIL_ASCENDING_WEST, RAIL_EAST_WEST, RAIL_NORTH_EAST, RAIL_NORTH_WEST, RAIL_SHAPES,
    RAIL_SOUTH_WEST, SNOW_UNITS, TORCH_WALL_RISE, TORCH_WALL_SHIFT, VINE_INSET_UNITS,
    WATER_SURFACE,
};
use world::tile::{PIXEL_GRID, TORCH_PIXELS, TORCH_TOP_ROW};

pub(crate) struct Face {
    pub(crate) normal: IVec3,
    pub(crate) facing: Facing,
    pub(crate) shade: f32,
    pub(crate) corners: [[i32; 3]; 4],
}

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

pub(crate) const WATER_SURFACE_UNITS: i32 = 14;
const _: () = assert!(WATER_SURFACE_UNITS as f32 == WATER_SURFACE * POSITION_UNITS as f32);

const SUNFLOWER_HEAD_X: u8 = 10;

pub(crate) const CROSS_SHADE: f32 = 1.0;

pub(crate) const TORCH_BOX: ([i32; 3], [i32; 3]) = (
    [TORCH_PIXELS.0 as i32, 0, TORCH_PIXELS.0 as i32],
    [
        TORCH_PIXELS.1 as i32,
        (PIXEL_GRID - TORCH_TOP_ROW) as i32,
        TORCH_PIXELS.1 as i32,
    ],
);

/// Яркость вершины по уровню AO (`vertex_ao`). Не 0.25-шаг по 0fps: вместе с `shade`
pub const AO_BRIGHTNESS: [f32; 4] = [0.5, 0.7, 0.85, 1.0];

pub const CUBE_FACE: u8 = 0;
pub const WATER_TOP: u8 = 6;
pub const WATER_SIDE: u8 = 7;
pub const WATER_SIDE_LOW: u8 = 11;
pub const WATER_BOTTOM: u8 = 15;
pub const CROSS: u8 = 16;
pub const TORCH_SIDE: u8 = 20;
pub const TORCH_TOP: u8 = 24;
pub const TORCH_WALL: u8 = 25;
pub const TORCH_WALL_PARTS: u8 = 5;
pub const LOG_BARK_X: u8 = 45;
pub const LOG_BARK_Z: u8 = 49;
pub const WATER_TOP_SLOPED: u8 = 53;
pub const WATER_SIDE_SLOPED: u8 = 54;
pub const SNOW_TOP: u8 = 58;
pub const SNOW_SIDE: u8 = 59;
pub const CACTUS: u8 = 63;
pub const VINE: u8 = 69;
pub const LILY: u8 = 79;
pub const SUNFLOWER_HEAD: u8 = 81;
pub const COCOA: u8 = 83;
pub const COCOA_PARTS: u8 = 11;
pub const RAIL: u8 = COCOA + COCOA_STAGES * 4 * COCOA_PARTS;
pub const POT_CROSS: u8 = RAIL + RAIL_SHAPES;
pub const POT_CROSS_PARTS: u8 = 4;
pub const LEVER: u16 = POT_CROSS as u16 + POT_CROSS_PARTS as u16;
pub const LEVER_PARTS: u16 = 6;
pub const HOOK: u16 = LEVER + crate::models::LEVER_STATES * LEVER_PARTS;
pub const HOOK_PARTS: u16 = 8;
pub const HOOK_ARM_PARTS: u16 = 6;
pub const WIRE: u16 = HOOK + crate::models::HOOK_STATES * HOOK_PARTS;
pub const WIRE_PARTS: u16 = 4;
pub const TEMPLATE_COUNT: usize = (WIRE + WIRE_PARTS) as usize;
const POT_PLANT_SPAN: (u8, u8) = (2, 14);
const POT_PLANT_BOTTOM: u8 = 4;

// shader.wgsl (vs_main) подставляет высоту из записи квада вместо этих значений y
pub const TOP_CORNER: u8 = 31;
pub const SIDE_TOP_CORNER: u8 = 30;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corner {
    pub pos: [u8; 3],
    pub uv: [u8; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Template {
    pub corners: [Corner; 4],
    pub shade: f32,
}

pub fn templates() -> &'static [Template; TEMPLATE_COUNT] {
    static TEMPLATES: LazyLock<[Template; TEMPLATE_COUNT]> = LazyLock::new(build);
    &TEMPLATES
}

fn cube_face(face: &Face, top: i32) -> Template {
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

fn sloped_water(face: &Face, marker: u8) -> Template {
    let mut template = cube_face(face, POSITION_UNITS);
    for (corner, [_, y, _]) in template.corners.iter_mut().zip(face.corners) {
        if y == 1 {
            corner.pos[1] = marker;
        }
    }
    template
}

fn torch_face(face: &Face) -> Template {
    let (lo, hi) = TORCH_BOX;
    let corners = face.corners.map(|[cx, cy, cz]| {
        let c = [cx, cy, cz];
        let p: [i32; 3] = std::array::from_fn(|i| if c[i] == 1 { hi[i] } else { lo[i] });
        let span = (TORCH_PIXELS.1 - TORCH_PIXELS.0) as i32;
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
                TORCH_TOP_ROW as i32 + (PIXEL_GRID as i32 - TORCH_TOP_ROW as i32) * (1 - cy),
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

fn bark_face(face: &Face, axis: usize) -> Template {
    let base = cube_face(face, POSITION_UNITS);
    let along = |t: &Template| {
        (0..4).all(|i| {
            (0..4).all(|j| {
                let (p, q) = (face.corners[i], face.corners[j]);
                let edge = p[axis] != q[axis] && (0..3).all(|k| k == axis || p[k] == q[k]);
                let (a, b) = (t.corners[i].uv, t.corners[j].uv);
                !edge || (a[1] != b[1] && a[0] == b[0])
            })
        })
    };
    (0..4)
        .map(|turn| {
            let mut t = base;
            for i in 0..4 {
                t.corners[i].uv = CORNER_UVS[(i + turn) % 4];
            }
            t
        })
        .find(|t| along(t))
        .expect("bark rotation")
}

fn torch_wall(turns: u32) -> impl Iterator<Item = Template> {
    let u = POSITION_UNITS;
    let sides = &FACES[2..];
    sides
        .iter()
        .map(torch_face)
        .chain(std::iter::once(torch_face(&FACES[0])))
        .map(move |mut t| {
            for c in &mut t.corners {
                let [x, y, z] = c.pos.map(i32::from);
                let slant =
                    TORCH_WALL_SHIFT as i32 * y / (PIXEL_GRID as i32 - TORCH_TOP_ROW as i32);
                let mut p = [
                    x - TORCH_PIXELS.0 as i32 + slant,
                    y + TORCH_WALL_RISE as i32,
                    z,
                ];
                for _ in 0..turns {
                    p = [u - p[2], p[1], p[0]];
                }
                c.pos = p.map(|v| v as u8);
            }
            t
        })
}

fn sunflower_head(face: &Face) -> Template {
    let mut template = cube_face(face, POSITION_UNITS);
    for corner in &mut template.corners {
        corner.pos[0] = SUNFLOWER_HEAD_X;
    }
    template
}

fn cactus_face(face: &Face) -> Template {
    let mut template = cube_face(face, POSITION_UNITS);
    let inset = i32::from(CACTUS_INSET_UNITS);
    let pull = |at_far: bool| {
        if at_far {
            (POSITION_UNITS - inset) as u8
        } else {
            inset as u8
        }
    };
    for (corner, cell) in template.corners.iter_mut().zip(face.corners) {
        if face.facing == Facing::Side {
            let axis = face.normal.abs().max_position();
            corner.pos[axis] = pull(cell[axis] == 1);
        } else {
            for axis in [0, 2] {
                corner.pos[axis] = pull(cell[axis] == 1);
            }
            corner.uv = corner.uv.map(|v| pull(v != 0));
        }
    }
    template
}

const COCOA_POD_TOP_UV: [i32; COCOA_STAGES as usize] = [4, 6, 7];
const COCOA_STEM_UV_U: [i32; COCOA_STAGES as usize] = [13, 12, 12];
const COCOA_POD_SIDE_ROW: i32 = 4;
const COCOA_STEM_UV_ROWS: (i32, i32) = (2, 4);

fn cocoa_box(
    face: &Face,
    (lo, hi): ([i32; 3], [i32; 3]),
    (u, v): ((i32, i32), (i32, i32)),
    turns: u32,
) -> Template {
    let units = POSITION_UNITS;
    let corners = std::array::from_fn(|i| {
        let mut p: [i32; 3] = std::array::from_fn(|k| {
            if face.corners[i][k] == 1 {
                hi[k]
            } else {
                lo[k]
            }
        });
        for _ in 0..turns {
            p = [units - p[2], p[1], p[0]];
        }
        let [cu, cv] = CORNER_UVS[i];
        Corner {
            pos: p.map(|c| c as u8),
            uv: [
                if cu == 0 { u.0 } else { u.1 } as u8,
                if cv == 0 { v.0 } else { v.1 } as u8,
            ],
        }
    });
    Template {
        corners,
        shade: face.shade,
    }
}

fn cocoa(stage: usize, direction: u32) -> Vec<Template> {
    let pod = COCOA_PODS[stage];
    let turns = (direction + 2) % 4;
    let (back, half) = (COCOA_BACK, pod.width / 2);
    let pod_box = (
        [8 - half, pod.bottom, back - pod.width],
        [8 + half, pod.top, back],
    );
    let pod_side = (
        (back - pod.width, back),
        (
            COCOA_POD_SIDE_ROW,
            COCOA_POD_SIDE_ROW + pod.top - pod.bottom,
        ),
    );
    let pod_end = ((0, COCOA_POD_TOP_UV[stage]), (0, COCOA_POD_TOP_UV[stage]));
    let stem_box = (
        [COCOA_STEM.0, pod.top, back - COCOA_STEM_DEPTH],
        [COCOA_STEM.1, POSITION_UNITS, back],
    );
    let stem_u = COCOA_STEM_UV_U[stage];
    let stem_uv = (
        (stem_u, stem_u + COCOA_STEM.1 - COCOA_STEM.0),
        COCOA_STEM_UV_ROWS,
    );
    let end = |face: &Face| cocoa_box(face, pod_box, pod_end, turns);
    let side = |face: &Face| cocoa_box(face, pod_box, pod_side, turns);
    let stem = |face: &Face| cocoa_box(face, stem_box, stem_uv, turns);
    [end(&FACES[0]), end(&FACES[1])]
        .into_iter()
        .chain(FACES[2..].iter().map(side))
        .chain(FACES[2..].iter().map(stem))
        .chain(std::iter::once(stem(&FACES[0])))
        .collect()
}

/// Рельс лежит на 1/16 над полом; поднятый край выше ещё на целый блок.
fn rail(shape: u8) -> Template {
    let turns = match shape {
        RAIL_EAST_WEST | RAIL_ASCENDING_EAST | RAIL_ASCENDING_WEST | RAIL_SOUTH_WEST => 1,
        RAIL_NORTH_WEST => 2,
        RAIL_NORTH_EAST => 3,
        _ => 0,
    };
    let lift = |x: i32, z: i32| {
        let raised = match shape {
            RAIL_ASCENDING_EAST => x == 1,
            RAIL_ASCENDING_WEST => x == 0,
            RAIL_ASCENDING_NORTH => z == 0,
            RAIL_ASCENDING_SOUTH => z == 1,
            _ => false,
        };
        1 + if raised { POSITION_UNITS } else { 0 }
    };
    let corners = std::array::from_fn(|i| {
        let [x, _, z] = FACES[0].corners[i];
        let (mut u, mut v) = (x, z);
        for _ in 0..turns {
            (u, v) = (v, 1 - u);
        }
        Corner {
            pos: [
                (x * POSITION_UNITS) as u8,
                lift(x, z) as u8,
                (z * POSITION_UNITS) as u8,
            ],
            uv: [(u * POSITION_UNITS) as u8, (v * POSITION_UNITS) as u8],
        }
    });
    Template {
        corners,
        shade: 1.0,
    }
}

fn vine_plane(face: &Face) -> Template {
    let mut template = cube_face(face, POSITION_UNITS);
    let axis = face.normal.abs().max_position();
    let inset = i32::from(VINE_INSET_UNITS);
    for (corner, cell) in template.corners.iter_mut().zip(face.corners) {
        corner.pos[axis] = if cell[axis] == 1 {
            (POSITION_UNITS - inset) as u8
        } else {
            inset as u8
        };
    }
    template
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
    for turns in [2, 0, 3, 1] {
        out.extend(torch_wall(turns));
    }
    out.extend([0, 1, 4, 5].map(|face| bark_face(&FACES[face], 0)));
    out.extend(FACES[..4].iter().map(|face| bark_face(face, 2)));
    out.push(sloped_water(&FACES[0], TOP_CORNER));
    out.extend(sides.iter().map(|f| sloped_water(f, SIDE_TOP_CORNER)));
    out.push(cube_face(&FACES[0], i32::from(SNOW_UNITS)));
    out.extend(sides.iter().map(|f| cube_face(f, i32::from(SNOW_UNITS))));
    out.extend(FACES.iter().map(cactus_face));
    for face in [&FACES[2], &FACES[3], &FACES[4], &FACES[5], &FACES[0]] {
        let front = vine_plane(face);
        let mut back = front;
        back.corners.reverse();
        out.extend([front, back]);
    }
    out.extend([&FACES[0], &FACES[1]].map(|f| cube_face(f, i32::from(LILY_UNITS))));
    out.extend([&FACES[2], &FACES[3]].map(sunflower_head));
    for stage in 0..COCOA_STAGES as usize {
        for direction in 0..4 {
            out.extend(cocoa(stage, direction));
        }
    }
    out.extend((0..RAIL_SHAPES).map(rail));
    let (lo, hi) = POT_PLANT_SPAN;
    for (a, b) in [([lo, lo], [hi, hi]), ([hi, lo], [lo, hi])] {
        let corner = |p: [u8; 2], y: u8, uv: [u8; 2]| Corner {
            pos: [p[0], y, p[1]],
            uv,
        };
        let front = [
            corner(a, POT_PLANT_BOTTOM, [0, u]),
            corner(b, POT_PLANT_BOTTOM, [u, u]),
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
    out.extend(crate::models::lever_templates());
    out.extend(crate::models::hook_templates());
    out.extend(crate::models::wire_templates());
    out.try_into().expect("template count")
}

pub fn pack_corner(c: Corner) -> u32 {
    let [x, y, z] = c.pos.map(u32::from);
    let [u, v] = c.uv.map(u32::from);
    x | y << 5 | z << 10 | u << 15 | v << 20
}

fn is_fluid_side(index: usize) -> bool {
    let Ok(index) = u8::try_from(index) else {
        return false;
    };
    (WATER_SIDE..WATER_BOTTOM).contains(&index) || (WATER_SIDE_SLOPED..SNOW_TOP).contains(&index)
}

pub const TABLE_BYTES: usize = TEMPLATE_COUNT * 32 + 16;

pub fn table_bytes() -> Vec<u8> {
    let mut words: Vec<u32> = Vec::with_capacity(TABLE_BYTES / 4);
    for (index, t) in templates().iter().enumerate() {
        words.extend(t.corners.map(pack_corner));
        words.extend([
            t.shade.to_bits(),
            f32::from(is_fluid_side(index)).to_bits(),
            0,
            0,
        ]);
    }
    words.extend(AO_BRIGHTNESS.map(f32::to_bits));
    words.into_iter().flat_map(u32::to_le_bytes).collect()
}
