use std::f32::consts::PI;

use glam::Mat3;
use world::mob::{horse_colour, horse_markings};

use super::{
    BoxPart, EntityTexture, LAID_FLAT, LEG_PHASES, MobInstance, head_rotation, leg_rotation, part,
};

pub const DONKEY_SCALE: f32 = 0.87;

const HORSE_SHEETS: [EntityTexture; 7] = [
    EntityTexture::HorseWhite,
    EntityTexture::HorseCreamy,
    EntityTexture::HorseChestnut,
    EntityTexture::HorseBrown,
    EntityTexture::HorseBlack,
    EntityTexture::HorseGray,
    EntityTexture::HorseDarkBrown,
];
const MARKING_SHEETS: [EntityTexture; 4] = [
    EntityTexture::MarkingsWhite,
    EntityTexture::MarkingsWhiteField,
    EntityTexture::MarkingsWhiteDots,
    EntityTexture::MarkingsBlackDots,
];
const LLAMA_SHEETS: [EntityTexture; 4] = [
    EntityTexture::LlamaCreamy,
    EntityTexture::LlamaWhite,
    EntityTexture::LlamaBrown,
    EntityTexture::LlamaGray,
];

const MARKINGS_GROW: f32 = 0.1;
const HEAD_TILT: f32 = PI / 6.0;
const DONKEY_EAR_SPLAY: f32 = 0.261_799_4;
const TAIL_BASE_TILT: f32 = -1.134_464;
const TAIL_TIP_TILT: f32 = -1.402_15;
const LEG_PIVOT_Y: f32 = 9.0;
const SHIN_Y: f32 = 7.0;
const HOOF_Y: f32 = 12.1;

struct Leg {
    x: f32,
    z: f32,
    texture: (f32, f32),
    thigh_x: f32,
    shin_x: f32,
    hoof_x: f32,
    front: bool,
}

const fn leg(x: f32, z: f32, texture: (f32, f32), xs: [f32; 3], front: bool) -> Leg {
    Leg {
        x,
        z,
        texture,
        thigh_x: xs[0],
        shin_x: xs[1],
        hoof_x: xs[2],
        front,
    }
}

const LEGS: [Leg; 4] = [
    leg(-4.0, 11.0, (96.0, 29.0), [-1.5, -1.0, -1.5], false),
    leg(4.0, 11.0, (78.0, 29.0), [-2.5, -2.0, -2.5], false),
    leg(-4.0, -8.0, (60.0, 29.0), [-1.1, -1.1, -1.6], true),
    leg(4.0, -8.0, (44.0, 29.0), [-1.9, -1.9, -2.4], true),
];

const NECK_PIVOT: [f32; 3] = [0.0, 4.0, -10.0];

pub fn horse_parts(instance: &MobInstance) -> Vec<BoxPart> {
    let sheet = HORSE_SHEETS[usize::from(horse_colour(instance.variant)) % HORSE_SHEETS.len()];
    let mut parts = equine_parts(instance, sheet, false);
    let markings = usize::from(horse_markings(instance.variant)) % (MARKING_SHEETS.len() + 1);
    if markings > 0 {
        let mut layer = equine_parts(instance, MARKING_SHEETS[markings - 1], false);
        for part in &mut layer {
            part.grow = MARKINGS_GROW;
        }
        parts.extend(layer);
    }
    parts
}

pub fn donkey_parts(instance: &MobInstance) -> Vec<BoxPart> {
    equine_parts(instance, EntityTexture::Donkey, true)
}

fn equine_parts(instance: &MobInstance, sheet: EntityTexture, long_ears: bool) -> Vec<BoxPart> {
    let head = Mat3::from_rotation_y(instance.head_yaw)
        * Mat3::from_rotation_x(HEAD_TILT - instance.head_pitch);
    let tail_pivot = [0.0, 3.0, 14.0];
    let mut parts = vec![
        part(
            sheet,
            (0.0, 34.0),
            [-5.0, -8.0, -19.0],
            [10.0, 10.0, 24.0],
            [0.0, 11.0, 9.0],
            Mat3::IDENTITY,
        ),
        part(
            sheet,
            (0.0, 0.0),
            [-2.5, -10.0, -1.5],
            [5.0, 5.0, 7.0],
            NECK_PIVOT,
            head,
        ),
        part(
            sheet,
            (24.0, 18.0),
            [-2.0, -10.0, -7.0],
            [4.0, 3.0, 6.0],
            NECK_PIVOT,
            head,
        ),
        part(
            sheet,
            (24.0, 27.0),
            [-2.0, -7.0, -6.5],
            [4.0, 2.0, 5.0],
            NECK_PIVOT,
            head,
        ),
        part(
            sheet,
            (0.0, 12.0),
            [-2.05, -9.8, -2.0],
            [4.0, 14.0, 8.0],
            NECK_PIVOT,
            head,
        ),
        part(
            sheet,
            (58.0, 0.0),
            [-1.0, -11.5, 5.0],
            [2.0, 16.0, 4.0],
            NECK_PIVOT,
            head,
        ),
        part(
            sheet,
            (44.0, 0.0),
            [-1.0, -1.0, 0.0],
            [2.0, 2.0, 3.0],
            tail_pivot,
            Mat3::from_rotation_x(TAIL_BASE_TILT),
        ),
        part(
            sheet,
            (38.0, 7.0),
            [-1.5, -2.0, 3.0],
            [3.0, 4.0, 7.0],
            tail_pivot,
            Mat3::from_rotation_x(TAIL_BASE_TILT),
        ),
        part(
            sheet,
            (24.0, 3.0),
            [-1.5, -4.5, 9.0],
            [3.0, 4.0, 7.0],
            tail_pivot,
            Mat3::from_rotation_x(TAIL_TIP_TILT),
        ),
    ];
    if long_ears {
        let splay = |angle: f32| Mat3::from_rotation_z(angle) * head;
        parts.push(part(
            sheet,
            (0.0, 12.0),
            [0.0, -16.0, 4.0],
            [2.0, 7.0, 1.0],
            NECK_PIVOT,
            splay(DONKEY_EAR_SPLAY),
        ));
        parts.push(part(
            sheet,
            (0.0, 12.0),
            [-2.0, -16.0, 4.0],
            [2.0, 7.0, 1.0],
            NECK_PIVOT,
            splay(-DONKEY_EAR_SPLAY),
        ));
    } else {
        parts.push(part(
            sheet,
            (0.0, 0.0),
            [0.45, -12.0, 4.0],
            [2.0, 3.0, 1.0],
            NECK_PIVOT,
            head,
        ));
        parts.push(part(
            sheet,
            (0.0, 0.0),
            [-2.45, -12.0, 4.0],
            [2.0, 3.0, 1.0],
            NECK_PIVOT,
            head,
        ));
    }
    for (leg, phase) in LEGS.into_iter().zip(LEG_PHASES) {
        let rotation = leg_rotation(instance, phase);
        let pivot = [leg.x, LEG_PIVOT_Y, leg.z];
        let (u, v) = leg.texture;
        let front = leg.front;
        let (leg_y, leg_size, leg_z) = if front {
            (-1.0, [3.0, 8.0, 4.0], -2.1)
        } else {
            (-2.0, [4.0, 9.0, 5.0], -2.5)
        };
        let (shin_v, shin_z, hoof_z) = if front {
            (v + 12.0, -1.6, -2.1)
        } else {
            (v + 14.0, -1.5, -2.0)
        };
        parts.push(part(
            sheet,
            (u, v),
            [leg.thigh_x, leg_y, leg_z],
            leg_size,
            pivot,
            rotation,
        ));
        parts.push(part(
            sheet,
            (u, shin_v),
            [leg.shin_x, SHIN_Y, shin_z],
            [3.0, 5.0, 3.0],
            pivot,
            rotation,
        ));
        parts.push(part(
            sheet,
            (u, v + 22.0),
            [leg.hoof_x, HOOF_Y, hoof_z],
            [4.0, 3.0, 4.0],
            pivot,
            rotation,
        ));
    }
    parts
}

pub fn llama_parts(instance: &MobInstance) -> Vec<BoxPart> {
    let sheet = LLAMA_SHEETS[usize::from(instance.variant) % LLAMA_SHEETS.len()];
    let head = head_rotation(instance);
    let head_pivot = [0.0, 7.0, -6.0];
    let mut parts = vec![
        part(
            sheet,
            (0.0, 0.0),
            [-2.0, -14.0, -10.0],
            [4.0, 4.0, 9.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (0.0, 14.0),
            [-4.0, -16.0, -6.0],
            [8.0, 18.0, 6.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (17.0, 0.0),
            [-4.0, -19.0, -4.0],
            [3.0, 3.0, 2.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (17.0, 0.0),
            [1.0, -19.0, -4.0],
            [3.0, 3.0, 2.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (29.0, 0.0),
            [-6.0, -10.0, -7.0],
            [12.0, 18.0, 10.0],
            [0.0, 5.0, 2.0],
            LAID_FLAT,
        ),
    ];
    let legs = [(-3.5, 6.0), (3.5, 6.0), (-3.5, -5.0), (3.5, -5.0)];
    for ((x, z), phase) in legs.into_iter().zip(LEG_PHASES) {
        parts.push(part(
            sheet,
            (29.0, 29.0),
            [-2.0, 0.0, -2.0],
            [4.0, 14.0, 4.0],
            [x, 10.0, z],
            leg_rotation(instance, phase),
        ));
    }
    parts
}
