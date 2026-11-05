//! Квады повёрнутых элементов рычага, крюка и нити: по одному шаблону на грань и состояние блока.

use glam::IVec3;
use world::block::{
    HOOK_POWERED, LEVER_CEILING, LEVER_FACING_MASK, LEVER_FLOOR, LEVER_MOUNT_MASK, LEVER_POWERED,
};
use world::models::{
    self, Element, Frame, hook_arm, hook_frame, hook_ring, lever_frame, lever_handle, rotate,
};

use crate::templates::{
    CORNER_UVS, Corner, FACES, HOOK, HOOK_PARTS, LEVER, LEVER_PARTS, Template, WIRE,
};

pub const LEVER_STATES: u16 = 24;
pub const HOOK_STATES: u16 = 8;
const MOUNTS: [u8; 3] = [0, LEVER_FLOOR, LEVER_CEILING];
const WIRE_HEIGHT: u8 = 2;
const WIRE_SHADE: f32 = 1.0;

pub fn lever_first(data: u8) -> u16 {
    let mount = match data & LEVER_MOUNT_MASK {
        LEVER_FLOOR => 1,
        LEVER_CEILING => 2,
        _ => 0,
    };
    let powered = u16::from(data & LEVER_POWERED != 0);
    let facing = u16::from(data & LEVER_FACING_MASK);
    LEVER + ((mount * 2 + powered) * 4 + facing) * LEVER_PARTS
}

pub fn hook_first(data: u8) -> u16 {
    let powered = u16::from(data & HOOK_POWERED != 0);
    let facing = u16::from(data & world::block::HOOK_FACING_MASK);
    HOOK + (powered * 4 + facing) * HOOK_PARTS
}

pub fn wire_first(along_z: bool) -> u16 {
    WIRE + 2 * u16::from(along_z)
}

fn shade_of(normal: IVec3) -> f32 {
    FACES
        .iter()
        .find(|f| f.normal == normal)
        .map_or(1.0, |f| f.shade)
}

/// Грань элемента: углы по `FACES`, повёрнуты элементом, затем рамкой блока; координаты округлены до 1/16.
fn element_face(element: &Element, frame: Frame, face: usize) -> Template {
    let reference = &FACES[face];
    let span: [f32; 3] = std::array::from_fn(|k| element.to[k] - element.from[k]);
    let uv = element.uv[face];
    let corner = |i: usize| {
        let at: [f32; 3] =
            std::array::from_fn(|k| element.from[k] + span[k] * reference.corners[i][k] as f32);
        let at = element.rotation.map_or(at, |r| rotate(at, r));
        let pos = frame.point(at).map(|v| v.round().clamp(0.0, 31.0) as u8);
        let [cu, cv] = CORNER_UVS[i];
        let u = if cu == 0 { uv[0] } else { uv[2] };
        let v = if cv == 0 { uv[1] } else { uv[3] };
        Corner {
            pos,
            uv: [u as u8, v as u8],
        }
    };
    Template {
        corners: std::array::from_fn(corner),
        shade: shade_of(frame.direction(reference.normal)),
    }
}

fn element_faces(element: &Element, frame: Frame) -> impl Iterator<Item = Template> + '_ {
    (0..models::FACE_COUNT).map(move |face| element_face(element, frame, face))
}

pub(crate) fn lever_templates() -> Vec<Template> {
    let mut out = Vec::new();
    for mount in MOUNTS {
        for powered in [0, LEVER_POWERED] {
            for facing in 0..4 {
                let data = mount | powered | facing;
                let handle = lever_handle(powered != 0);
                out.extend(element_faces(&handle, lever_frame(data)));
            }
        }
    }
    out
}

pub(crate) fn hook_templates() -> Vec<Template> {
    let mut out = Vec::new();
    for powered in [0, HOOK_POWERED] {
        for facing in 0..4 {
            let data = powered | facing;
            let frame = hook_frame(data);
            out.extend(element_faces(&hook_arm(powered != 0), frame));
            let ring = hook_ring(powered != 0);
            out.extend([4, 5].map(|face| element_face(&ring, frame, face)));
        }
    }
    out
}

/// Нить — плоский квад в клетке со всей плиткой; вдоль x и вдоль z, лицом вверх и вниз.
pub(crate) fn wire_templates() -> Vec<Template> {
    let mut out = Vec::new();
    for along_z in [false, true] {
        let front: [Corner; 4] = std::array::from_fn(|i| {
            let [x, _, z] = FACES[0].corners[i];
            let (x, z) = (x as u8 * 16, z as u8 * 16);
            let pos = if along_z {
                [16 - z, WIRE_HEIGHT, x]
            } else {
                [x, WIRE_HEIGHT, z]
            };
            Corner {
                pos,
                uv: CORNER_UVS[i],
            }
        });
        let mut back = front;
        back.reverse();
        out.extend([front, back].map(|corners| Template {
            corners,
            shade: WIRE_SHADE,
        }));
    }
    out
}

#[cfg(test)]
mod tests;
