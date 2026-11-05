//! Модели рычага и крюка растяжки по `block/lever*.json` и `block/tripwire_hook*.json` 1.12: элементы
//! в пикселях 0..16, повёрнутый элемент — отдельными квадами, остальные — коробками.

use glam::{IVec3, Vec3};

use crate::block::{
    Block, HOOK_FACING_MASK, LEVER_CEILING, LEVER_FACING_MASK, LEVER_FLOOR, LEVER_MOUNT_MASK,
};
use crate::boxes::{BOX_UNITS, VoxelBox};

/// Грани элемента в порядке `FACES` мешера: верх, низ, +x, −x, +z, −z; uv — `[u0, v0, u1, v1]`.
pub const FACE_COUNT: usize = 6;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rotation {
    pub origin: [f32; 3],
    pub axis: usize,
    pub angle: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Element {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub rotation: Option<Rotation>,
    pub uv: [[f32; 4]; FACE_COUNT],
}

/// Поворот модели на четверть-обороты: образы осей канонической модели (y — от опоры, z — «вверх» у стены).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Frame {
    pub x: IVec3,
    pub y: IVec3,
    pub z: IVec3,
}

impl Frame {
    fn of(y: IVec3, z: IVec3) -> Self {
        Self {
            x: y.cross(z),
            y,
            z,
        }
    }

    pub fn point(self, p: [f32; 3]) -> [f32; 3] {
        let c = Vec3::splat(f32::from(BOX_UNITS) / 2.0);
        let d = Vec3::from(p) - c;
        (c + self.x.as_vec3() * d.x + self.y.as_vec3() * d.y + self.z.as_vec3() * d.z).to_array()
    }

    pub fn direction(self, v: IVec3) -> IVec3 {
        self.x * v.x + self.y * v.y + self.z * v.z
    }
}

pub fn lever_frame(data: u8) -> Frame {
    let facing = Block::Lever.facing_dir(data & LEVER_FACING_MASK);
    match data & LEVER_MOUNT_MASK {
        LEVER_FLOOR => Frame::of(IVec3::Y, facing),
        LEVER_CEILING => Frame::of(IVec3::NEG_Y, facing),
        _ => Frame::of(facing, IVec3::Y),
    }
}

pub fn hook_frame(data: u8) -> Frame {
    Frame::of(
        Block::TripwireHook.facing_dir(data & HOOK_FACING_MASK),
        IVec3::Y,
    )
}

pub const LEVER_BASE: ([f32; 3], [f32; 3]) = ([5.0, 0.0, 4.0], [11.0, 3.0, 12.0]);
pub const HOOK_PLATE: ([f32; 3], [f32; 3]) = ([6.0, 0.0, 4.0], [10.0, 2.0, 12.0]);

const LEVER_LEAN: f32 = 45.0;
const HOOK_DROOPED: f32 = 45.0;
const HOOK_RAISED: f32 = 40.0;
const HOOK_RING_TILT: f32 = 60.0;

fn stick(
    from: [f32; 3],
    to: [f32; 3],
    pivot: [f32; 3],
    angle: f32,
    caps: [f32; 4],
    side: [f32; 4],
) -> Element {
    Element {
        from,
        to,
        rotation: Some(Rotation {
            origin: pivot,
            axis: 0,
            angle,
        }),
        uv: [caps, caps, side, side, side, side],
    }
}

/// Ручка рычага: вертикальная палка, наклон вдоль канонической оси z (у стены — вверх при включении).
pub fn lever_handle(powered: bool) -> Element {
    let angle = if powered { LEVER_LEAN } else { -LEVER_LEAN };
    stick(
        [7.0, 1.0, 7.0],
        [9.0, 11.0, 9.0],
        [8.0, 1.0, 8.0],
        angle,
        [7.0, 6.0, 9.0, 8.0],
        [7.0, 6.0, 9.0, 16.0],
    )
}

/// Палка крюка из досок: у спокойного крюка идёт вниз и наружу, у сработавшего короче и смотрит вверх.
pub fn hook_arm(powered: bool) -> Element {
    let (length, angle) = if powered {
        (4.0, HOOK_RAISED)
    } else {
        (7.0, -HOOK_DROOPED)
    };
    stick(
        [7.0, 2.0, 7.0],
        [9.0, 2.0 + length, 9.0],
        [8.0, 2.0, 8.0],
        angle,
        [7.0, 7.0, 9.0, 9.0],
        [7.0, 0.0, 9.0, 7.0],
    )
}

/// Железное кольцо на конце палки: плоское лежит при спокойном крюке, у сработавшего стоит наклонно.
pub fn hook_ring(powered: bool) -> Element {
    let arm = hook_arm(powered);
    let rotation = arm.rotation.expect("the arm is rotated");
    let end = rotate([8.0, arm.to[1], 8.0], rotation);
    let tilt = if powered { -HOOK_RING_TILT } else { 0.0 };
    Element {
        from: [end[0] - 3.0, end[1] - 3.0, end[2] - 0.5],
        to: [end[0] + 3.0, end[1] + 3.0, end[2] + 0.5],
        rotation: Some(Rotation {
            origin: end,
            axis: 0,
            angle: tilt,
        }),
        uv: [HOOK_RING_UV; FACE_COUNT],
    }
}

pub const HOOK_RING_UV: [f32; 4] = [5.0, 3.0, 11.0, 9.0];

pub fn rotate(p: [f32; 3], rotation: Rotation) -> [f32; 3] {
    let (sin, cos) = rotation.angle.to_radians().sin_cos();
    let mut d = Vec3::from(p) - Vec3::from(rotation.origin);
    let (a, b) = match rotation.axis {
        0 => (1, 2),
        1 => (2, 0),
        _ => (0, 1),
    };
    let (u, v) = (d[a], d[b]);
    d[a] = u * cos - v * sin;
    d[b] = u * sin + v * cos;
    (Vec3::from(rotation.origin) + d).to_array()
}

pub fn corners(from: [f32; 3], to: [f32; 3]) -> [[f32; 3]; 8] {
    std::array::from_fn(|i| std::array::from_fn(|k| if i >> k & 1 == 1 { to[k] } else { from[k] }))
}

fn bounds(points: impl Iterator<Item = [f32; 3]>) -> VoxelBox {
    let unit = f32::from(BOX_UNITS);
    let (mut lo, mut hi) = (Vec3::splat(unit), Vec3::ZERO);
    for p in points {
        lo = lo.min(Vec3::from(p));
        hi = hi.max(Vec3::from(p));
    }
    let clamp = |v: f32| v.clamp(0.0, unit) as u8;
    VoxelBox::new(
        lo.to_array().map(|v| clamp(v.floor())),
        hi.to_array().map(|v| clamp(v.ceil())),
    )
}

pub fn framed_box(frame: Frame, (from, to): ([f32; 3], [f32; 3])) -> VoxelBox {
    bounds(corners(from, to).into_iter().map(|p| frame.point(p)))
}

pub fn framed_element(frame: Frame, element: &Element) -> VoxelBox {
    bounds(corners(element.from, element.to).into_iter().map(|p| {
        let p = element.rotation.map_or(p, |r| rotate(p, r));
        frame.point(p)
    }))
}

/// Основание и область выбора: у стены 6×6×10, у пола и потолка 8×10×8, как `getBoundingBox` 1.12.
pub fn lever_boxes(data: u8) -> [VoxelBox; 2] {
    let frame = lever_frame(data);
    let reach = if data & LEVER_MOUNT_MASK == 0 {
        ([5.0, 0.0, 3.0], [11.0, 6.0, 13.0])
    } else {
        ([4.0, 0.0, 4.0], [12.0, 10.0, 12.0])
    };
    [framed_box(frame, LEVER_BASE), framed_box(frame, reach)]
}

pub fn hook_boxes(data: u8) -> [VoxelBox; 2] {
    let frame = hook_frame(data);
    [
        framed_box(frame, HOOK_PLATE),
        framed_box(frame, ([5.0, 0.0, 3.0], [11.0, 6.0, 13.0])),
    ]
}

#[cfg(test)]
mod tests;
