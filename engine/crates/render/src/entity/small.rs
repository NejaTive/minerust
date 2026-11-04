use std::f32::consts::PI;

use glam::Mat3;
use world::mob::RABBIT_COLOURS;

use super::{BoxPart, EntityTexture, LAID_FLAT, MobInstance, head_rotation, part};

const RABBIT_SHEETS: [EntityTexture; RABBIT_COLOURS as usize] = [
    EntityTexture::RabbitBrown,
    EntityTexture::RabbitWhite,
    EntityTexture::RabbitBlack,
    EntityTexture::RabbitSplotched,
    EntityTexture::RabbitGold,
    EntityTexture::RabbitSalt,
];

pub const RABBIT_SCALE: f32 = 0.6;

const RABBIT_BODY_TILT: f32 = -0.349_065_85;
const RABBIT_EAR_SPLAY: f32 = 0.261_799_4;
const RABBIT_FOOT_JUMP: f32 = 50.0 * PI / 180.0;
const RABBIT_THIGH_REST: f32 = -21.0 * PI / 180.0;
const RABBIT_ARM_JUMP: f32 = -40.0 * PI / 180.0;
const RABBIT_ARM_REST: f32 = -11.0 * PI / 180.0;

pub fn rabbit_parts(instance: &MobInstance) -> Vec<BoxPart> {
    let sheet = RABBIT_SHEETS[usize::from(instance.variant) % RABBIT_SHEETS.len()];
    let jump = (instance.hop * PI).sin();
    let foot = Mat3::from_rotation_x(jump * RABBIT_FOOT_JUMP);
    let thigh = Mat3::from_rotation_x(jump * RABBIT_FOOT_JUMP + RABBIT_THIGH_REST);
    let arm = Mat3::from_rotation_x(jump * RABBIT_ARM_JUMP + RABBIT_ARM_REST);
    let head = head_rotation(instance);
    let head_pivot = [0.0, 16.0, -1.0];
    let hind_pivot = |x: f32| [x, 17.5, 3.7];
    vec![
        part(
            sheet,
            (26.0, 24.0),
            [-1.0, 5.5, -3.7],
            [2.0, 1.0, 7.0],
            hind_pivot(3.0),
            foot,
        ),
        part(
            sheet,
            (8.0, 24.0),
            [-1.0, 5.5, -3.7],
            [2.0, 1.0, 7.0],
            hind_pivot(-3.0),
            foot,
        ),
        part(
            sheet,
            (30.0, 15.0),
            [-1.0, 0.0, 0.0],
            [2.0, 4.0, 5.0],
            hind_pivot(3.0),
            thigh,
        ),
        part(
            sheet,
            (16.0, 15.0),
            [-1.0, 0.0, 0.0],
            [2.0, 4.0, 5.0],
            hind_pivot(-3.0),
            thigh,
        ),
        part(
            sheet,
            (0.0, 0.0),
            [-3.0, -2.0, -10.0],
            [6.0, 5.0, 10.0],
            [0.0, 19.0, 8.0],
            Mat3::from_rotation_x(RABBIT_BODY_TILT),
        ),
        part(
            sheet,
            (8.0, 15.0),
            [-1.0, 0.0, -1.0],
            [2.0, 7.0, 2.0],
            [3.0, 17.0, -1.0],
            arm,
        ),
        part(
            sheet,
            (0.0, 15.0),
            [-1.0, 0.0, -1.0],
            [2.0, 7.0, 2.0],
            [-3.0, 17.0, -1.0],
            arm,
        ),
        part(
            sheet,
            (32.0, 0.0),
            [-2.5, -4.0, -5.0],
            [5.0, 4.0, 5.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (52.0, 0.0),
            [-2.5, -9.0, -1.0],
            [2.0, 5.0, 1.0],
            head_pivot,
            head * Mat3::from_rotation_y(-RABBIT_EAR_SPLAY),
        ),
        part(
            sheet,
            (58.0, 0.0),
            [0.5, -9.0, -1.0],
            [2.0, 5.0, 1.0],
            head_pivot,
            head * Mat3::from_rotation_y(RABBIT_EAR_SPLAY),
        ),
        part(
            sheet,
            (52.0, 6.0),
            [-1.5, -1.5, 0.0],
            [3.0, 3.0, 2.0],
            [0.0, 20.0, 7.0],
            Mat3::from_rotation_x(RABBIT_BODY_TILT),
        ),
        part(
            sheet,
            (32.0, 9.0),
            [-0.5, -2.5, -5.5],
            [1.0, 1.0, 1.0],
            head_pivot,
            head,
        ),
    ]
}

const OCELOT_TAIL_TILT: f32 = 0.9;
const OCELOT_TAIL_TIP_TILT: f32 = 1.727_876_1;
const OCELOT_TAIL_TIP_SWING: f32 = PI / 10.0;
const OCELOT_LEG_FREQUENCY: f32 = 0.6662;

pub fn ocelot_parts(instance: &MobInstance) -> Vec<BoxPart> {
    let sheet = EntityTexture::Ocelot;
    let head = head_rotation(instance);
    let head_pivot = [0.0, 15.0, -9.0];
    let swing = |phase: f32| {
        Mat3::from_rotation_x(
            (instance.limb_phase * OCELOT_LEG_FREQUENCY + phase).cos() * instance.limb_swing,
        )
    };
    let tail_tip = OCELOT_TAIL_TIP_TILT
        + OCELOT_TAIL_TIP_SWING * instance.limb_phase.cos() * instance.limb_swing;
    vec![
        part(
            sheet,
            (0.0, 0.0),
            [-2.5, -2.0, -3.0],
            [5.0, 4.0, 5.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (0.0, 24.0),
            [-1.5, 0.0, -4.0],
            [3.0, 2.0, 2.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (0.0, 10.0),
            [-2.0, -3.0, 0.0],
            [1.0, 1.0, 2.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (6.0, 10.0),
            [1.0, -3.0, 0.0],
            [1.0, 1.0, 2.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (20.0, 0.0),
            [-2.0, 3.0, -8.0],
            [4.0, 16.0, 6.0],
            [0.0, 12.0, -10.0],
            LAID_FLAT,
        ),
        part(
            sheet,
            (0.0, 15.0),
            [-0.5, 0.0, 0.0],
            [1.0, 8.0, 1.0],
            [0.0, 15.0, 8.0],
            Mat3::from_rotation_x(OCELOT_TAIL_TILT),
        ),
        part(
            sheet,
            (4.0, 15.0),
            [-0.5, 0.0, 0.0],
            [1.0, 8.0, 1.0],
            [0.0, 20.0, 14.0],
            Mat3::from_rotation_x(tail_tip),
        ),
        part(
            sheet,
            (8.0, 13.0),
            [-1.0, 0.0, 1.0],
            [2.0, 6.0, 2.0],
            [1.1, 18.0, 5.0],
            swing(0.0),
        ),
        part(
            sheet,
            (8.0, 13.0),
            [-1.0, 0.0, 1.0],
            [2.0, 6.0, 2.0],
            [-1.1, 18.0, 5.0],
            swing(0.3),
        ),
        part(
            sheet,
            (40.0, 0.0),
            [-1.0, 0.0, 0.0],
            [2.0, 10.0, 2.0],
            [1.2, 13.8, -5.0],
            swing(PI + 0.3),
        ),
        part(
            sheet,
            (40.0, 0.0),
            [-1.0, 0.0, 0.0],
            [2.0, 10.0, 2.0],
            [-1.2, 13.8, -5.0],
            swing(PI),
        ),
    ]
}
