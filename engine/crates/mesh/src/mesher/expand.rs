use crate::mesh::{POSITION_UNITS, Quad, light_brightness, quantize_shade};
use crate::mesher::MeshData;
use crate::templates::{AO_BRIGHTNESS, SIDE_TOP_CORNER, TOP_CORNER, templates};
use world::boxes::face_axes;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vertex {
    position: [u16; 3],
    uv: [u8; 2],
    shade: u8,
    layer: u16,
    light: [u8; 2],
}

impl Vertex {
    pub fn position(self) -> [f32; 3] {
        self.position.map(|p| f32::from(p) / POSITION_UNITS as f32)
    }

    pub fn uv(self) -> [f32; 2] {
        self.uv.map(|c| f32::from(c) / POSITION_UNITS as f32)
    }

    pub fn shade_byte(self) -> u8 {
        self.shade
    }

    pub fn shade(self) -> f32 {
        f32::from(self.shade) / 255.0
    }

    pub fn layer(self) -> u16 {
        self.layer
    }

    pub fn sun_halves(self) -> u8 {
        self.light[0]
    }

    pub fn block_halves(self) -> u8 {
        self.light[1]
    }

    pub fn brightness(self, darkness: f32) -> f32 {
        self.shade() * light_brightness(self.light[0], self.light[1], darkness)
    }
}

pub fn vertices(data: &MeshData) -> Vec<Vertex> {
    data.quads
        .iter()
        .flat_map(|quad| {
            let template = &templates()[usize::from(quad.template_index())];
            let [cx, cy, cz] = quad.cell().map(|c| u16::from(c) * POSITION_UNITS as u16);
            let quad = *quad;
            std::array::from_fn::<_, 4, _>(move |i| {
                let corner = template.corners[i];
                let [mut x, mut y, mut z] = corner.pos.map(u16::from);
                let mut uv = corner.uv;
                let ao = quad.ao()[i];
                if quad.is_boxed() {
                    (x, y, z, uv) = boxed_corner(quad, usize::from(quad.template()), [x, y, z], uv);
                }
                if y == u16::from(TOP_CORNER) || y == u16::from(SIDE_TOP_CORNER) {
                    let top = quad.heights()[i];
                    if y == u16::from(SIDE_TOP_CORNER) {
                        uv[1] = POSITION_UNITS as u8 - top;
                    }
                    y = u16::from(top);
                }
                Vertex {
                    position: [cx + x, cy + y, cz + z],
                    uv,
                    shade: quantize_shade(template.shade * AO_BRIGHTNESS[usize::from(ao)]),
                    layer: quad.layer(),
                    light: [quad.sun_halves()[i], quad.block_halves()[i]],
                }
            })
        })
        .collect()
}

pub fn indices(data: &MeshData) -> Vec<u16> {
    data.quads
        .iter()
        .enumerate()
        .flat_map(|(q, quad)| {
            let first = (q * 4) as u16;
            let order: [u16; 6] = if quad.flipped() {
                [1, 2, 3, 1, 3, 0]
            } else {
                [0, 1, 2, 0, 2, 3]
            };
            order.map(|i| first + i)
        })
        .collect()
}

fn boxed_corner(quad: Quad, face: usize, pos: [u16; 3], uv: [u8; 2]) -> (u16, u16, u16, [u8; 2]) {
    let (rect, plane) = quad.box_rect();
    let normal = [1, 1, 0, 0, 2, 2][face];
    let (ua, va) = face_axes(normal);
    let units = POSITION_UNITS as u16;
    let mut at = pos;
    at[normal] = u16::from(plane);
    at[ua] = u16::from(if pos[ua] == 0 { rect.0 } else { rect.1 });
    at[va] = u16::from(if pos[va] == 0 { rect.2 } else { rect.3 });
    let pick = |axis: usize, template_uv: u8| {
        let along = at[axis];
        (if u16::from(template_uv) == pos[axis] {
            along
        } else {
            units - along
        }) as u8
    };
    (at[0], at[1], at[2], [pick(ua, uv[0]), pick(va, uv[1])])
}
