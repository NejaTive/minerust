use std::f32::consts::{FRAC_PI_2, PI, TAU};

use glam::{Mat3, Vec3};
use world::mob::PARROT_COLOURS;

use super::{BoxPart, EntityTexture, MobInstance, head_rotation, mirrored, part};

const PARROT_SHEETS: [EntityTexture; PARROT_COLOURS as usize] = [
    EntityTexture::ParrotRedBlue,
    EntityTexture::ParrotBlue,
    EntityTexture::ParrotGreen,
    EntityTexture::ParrotYellowBlue,
    EntityTexture::ParrotGrey,
];

const PARROT_FLYING_FROM: f32 = 0.5;
const PARROT_BOB: f32 = 0.3;
const PARROT_BODY_TILT: f32 = 0.4937;
const PARROT_TAIL_TILT: f32 = 1.015;
const PARROT_TAIL_SWING: f32 = 0.3;
const PARROT_WING_TILT: f32 = -0.6981;
const PARROT_WING_REST: f32 = 0.0873;
const PARROT_FEATHER_TILT: f32 = -0.2214;
const PARROT_LEG_TILT: f32 = -0.0299;
const PARROT_LEG_TUCKED: f32 = 0.6981;
const PARROT_LEG_SWING: f32 = 1.4;
const PARROT_LEG_FREQUENCY: f32 = 0.6662;

pub fn parrot_parts(instance: &MobInstance) -> Vec<BoxPart> {
    let sheet = PARROT_SHEETS[usize::from(instance.variant) % PARROT_SHEETS.len()];
    let flying = instance.flap_amount > PARROT_FLYING_FROM;
    let flap = ((instance.flap_phase.sin() + 1.0) * instance.flap_amount).max(0.0);
    let bob = flap * PARROT_BOB;
    let head = head_rotation(instance);
    let head_pivot = Vec3::new(0.0, 15.69 + bob, -2.76);
    let walk = instance.limb_phase * PARROT_LEG_FREQUENCY;
    let leg_tilt = |phase: f32| {
        PARROT_LEG_TILT
            + if flying {
                PARROT_LEG_TUCKED
            } else {
                (walk + phase).cos() * PARROT_LEG_SWING * instance.limb_swing
            }
    };
    let wing = |side: f32| {
        Mat3::from_rotation_z(side * (PARROT_WING_REST + flap))
            * Mat3::from_rotation_x(PARROT_WING_TILT)
    };
    let on_head =
        |texture: (f32, f32), origin: [f32; 3], size: [f32; 3], offset: [f32; 3], tilt: f32| {
            part(
                sheet,
                texture,
                origin,
                size,
                (head_pivot + head * Vec3::from(offset)).to_array(),
                head * Mat3::from_rotation_x(tilt),
            )
        };
    vec![
        part(
            sheet,
            (2.0, 8.0),
            [-1.5, 0.0, -1.5],
            [3.0, 6.0, 3.0],
            [0.0, 16.5 + bob, -3.0],
            Mat3::from_rotation_x(PARROT_BODY_TILT),
        ),
        part(
            sheet,
            (22.0, 1.0),
            [-1.5, -1.0, -1.0],
            [3.0, 4.0, 1.0],
            [0.0, 21.07 + bob, 1.16],
            Mat3::from_rotation_x(
                PARROT_TAIL_TILT + walk.cos() * PARROT_TAIL_SWING * instance.limb_swing,
            ),
        ),
        part(
            sheet,
            (19.0, 8.0),
            [-0.5, 0.0, -1.5],
            [1.0, 5.0, 3.0],
            [1.5, 16.94 + bob, -2.76],
            wing(-1.0),
        ),
        mirrored(part(
            sheet,
            (19.0, 8.0),
            [-0.5, 0.0, -1.5],
            [1.0, 5.0, 3.0],
            [-1.5, 16.94 + bob, -2.76],
            wing(1.0),
        )),
        part(
            sheet,
            (2.0, 2.0),
            [-1.0, -1.5, -1.0],
            [2.0, 3.0, 2.0],
            head_pivot.to_array(),
            head,
        ),
        on_head(
            (10.0, 0.0),
            [-1.0, -0.5, -2.0],
            [2.0, 1.0, 4.0],
            [0.0, -2.0, -1.0],
            0.0,
        ),
        on_head(
            (11.0, 7.0),
            [-0.5, -1.0, -0.5],
            [1.0, 2.0, 1.0],
            [0.0, -0.5, -1.5],
            0.0,
        ),
        on_head(
            (16.0, 7.0),
            [-0.5, 0.0, -0.5],
            [1.0, 2.0, 1.0],
            [0.0, -1.75, -2.45],
            0.0,
        ),
        on_head(
            (2.0, 18.0),
            [0.0, -4.0, -2.0],
            [0.0, 5.0, 4.0],
            [0.0, -2.15, 0.15],
            PARROT_FEATHER_TILT,
        ),
        part(
            sheet,
            (14.0, 18.0),
            [-0.5, 0.0, -0.5],
            [1.0, 2.0, 1.0],
            [1.0, 22.0 + bob, -1.05],
            Mat3::from_rotation_x(leg_tilt(0.0)),
        ),
        mirrored(part(
            sheet,
            (14.0, 18.0),
            [-0.5, 0.0, -0.5],
            [1.0, 2.0, 1.0],
            [-1.0, 22.0 + bob, -1.05],
            Mat3::from_rotation_x(leg_tilt(PI)),
        )),
    ]
}

const SQUID_TENTACLES: u32 = 8;
const SQUID_HANG: f32 = 11.2;
const SQUID_CENTER_Y: f32 = 16.0;

pub fn squid_parts(instance: &MobInstance) -> Vec<BoxPart> {
    let sheet = EntityTexture::Squid;
    let mut parts = vec![part(
        sheet,
        (0.0, 0.0),
        [-6.0, -8.0, -6.0],
        [12.0, 16.0, 12.0],
        [0.0, 8.0, 0.0],
        Mat3::IDENTITY,
    )];
    for index in 0..SQUID_TENTACLES {
        let angle = index as f32 * TAU / SQUID_TENTACLES as f32;
        parts.push(part(
            sheet,
            (48.0, 0.0),
            [-1.0, 0.0, -1.0],
            [2.0, 18.0, 2.0],
            [angle.cos() * 5.0, 15.0, angle.sin() * 5.0],
            Mat3::from_rotation_y(FRAC_PI_2 - angle) * Mat3::from_rotation_x(instance.tentacle),
        ));
    }
    let tilt = Mat3::from_rotation_x(instance.pitch) * Mat3::from_rotation_y(instance.spin);
    let center = Vec3::new(0.0, SQUID_CENTER_Y, 0.0);
    for part in &mut parts {
        part.pivot = center + tilt * (part.pivot + Vec3::Y * SQUID_HANG - center);
        part.rotation = tilt * part.rotation;
    }
    parts
}
