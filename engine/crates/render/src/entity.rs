use std::f32::consts::{FRAC_PI_2, PI};

use glam::{Mat3, Vec2, Vec3};
use wgpu::util::DeviceExt;
use world::pack::EntityImages;

pub const VERTEX_FLOATS: usize = 8;
pub const VERTEX_BYTES: usize = VERTEX_FLOATS * 4;
pub const BLOB_FLOATS: usize = 6;
pub const BLOB_BYTES: usize = BLOB_FLOATS * 4;
const MUSHROOM_UNITS: Vec2 = Vec2::new(16.0, 16.0);
const PIXELS_PER_BLOCK: f32 = 16.0;
const MODEL_FEET: f32 = 24.0;
const CULL_RADIUS: f32 = 1.6;
const SHADOW_DROP: f32 = 2.0;
const CULL_HEIGHT: f32 = 2.6;
const LEG_SWING: f32 = 1.4;
const LEG_FREQUENCY: f32 = 0.6662;
const LAID_FLAT: Mat3 = Mat3::from_cols(
    Vec3::new(1.0, 0.0, 0.0),
    Vec3::new(0.0, 0.0, 1.0),
    Vec3::new(0.0, -1.0, 0.0),
);

const ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32, 3 => Float32x2];

pub fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: VERTEX_BYTES as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRIBUTES,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModelKind {
    Pig,
    Chicken,
    Cow,
    Sheep,
    Mooshroom,
    PolarBear,
    Wolf,
    Horse,
    Donkey,
    Llama,
    Rabbit,
    Ocelot,
}

#[derive(Clone, Copy, Debug)]
pub struct ShadowPatch {
    pub min: Vec2,
    pub max: Vec2,
    pub y: f32,
    pub opacity: f32,
}

#[derive(Clone, Debug)]
pub struct MobInstance {
    pub kind: ModelKind,
    pub variant: u8,
    pub feet: Vec3,
    pub yaw: f32,
    pub head_yaw: f32,
    pub head_pitch: f32,
    pub limb_phase: f32,
    pub limb_swing: f32,
    pub flap_phase: f32,
    pub flap_amount: f32,
    pub hop: f32,
    pub sky_level: f32,
    pub block_level: f32,
    pub shadow_radius: f32,
    pub shadow: Vec<ShadowPatch>,
}

impl MobInstance {
    pub fn bounds(&self) -> (Vec3, Vec3) {
        (
            self.feet - Vec3::new(CULL_RADIUS, SHADOW_DROP, CULL_RADIUS),
            self.feet + Vec3::new(CULL_RADIUS, CULL_HEIGHT, CULL_RADIUS),
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EntityTexture {
    Pig,
    Chicken,
    Cow,
    Sheep,
    SheepFur,
    Mooshroom,
    PolarBear,
    Wolf,
    Mushroom,
    HorseWhite,
    HorseCreamy,
    HorseChestnut,
    HorseBrown,
    HorseBlack,
    HorseGray,
    HorseDarkBrown,
    MarkingsWhite,
    MarkingsWhiteField,
    MarkingsWhiteDots,
    MarkingsBlackDots,
    Donkey,
    LlamaCreamy,
    LlamaWhite,
    LlamaBrown,
    LlamaGray,
    RabbitBrown,
    RabbitWhite,
    RabbitBlack,
    RabbitSplotched,
    RabbitGold,
    RabbitSalt,
    Ocelot,
}

impl EntityTexture {
    pub const ALL: [EntityTexture; TEXTURES] = [
        EntityTexture::Pig,
        EntityTexture::Chicken,
        EntityTexture::Cow,
        EntityTexture::Sheep,
        EntityTexture::SheepFur,
        EntityTexture::Mooshroom,
        EntityTexture::PolarBear,
        EntityTexture::Wolf,
        EntityTexture::Mushroom,
        EntityTexture::HorseWhite,
        EntityTexture::HorseCreamy,
        EntityTexture::HorseChestnut,
        EntityTexture::HorseBrown,
        EntityTexture::HorseBlack,
        EntityTexture::HorseGray,
        EntityTexture::HorseDarkBrown,
        EntityTexture::MarkingsWhite,
        EntityTexture::MarkingsWhiteField,
        EntityTexture::MarkingsWhiteDots,
        EntityTexture::MarkingsBlackDots,
        EntityTexture::Donkey,
        EntityTexture::LlamaCreamy,
        EntityTexture::LlamaWhite,
        EntityTexture::LlamaBrown,
        EntityTexture::LlamaGray,
        EntityTexture::RabbitBrown,
        EntityTexture::RabbitWhite,
        EntityTexture::RabbitBlack,
        EntityTexture::RabbitSplotched,
        EntityTexture::RabbitGold,
        EntityTexture::RabbitSalt,
        EntityTexture::Ocelot,
    ];

    fn units(self) -> Vec2 {
        match self {
            EntityTexture::PolarBear => Vec2::new(128.0, 64.0),
            EntityTexture::Mushroom => MUSHROOM_UNITS,
            EntityTexture::LlamaCreamy
            | EntityTexture::LlamaWhite
            | EntityTexture::LlamaBrown
            | EntityTexture::LlamaGray => Vec2::new(128.0, 64.0),
            EntityTexture::HorseWhite
            | EntityTexture::HorseCreamy
            | EntityTexture::HorseChestnut
            | EntityTexture::HorseBrown
            | EntityTexture::HorseBlack
            | EntityTexture::HorseGray
            | EntityTexture::HorseDarkBrown
            | EntityTexture::MarkingsWhite
            | EntityTexture::MarkingsWhiteField
            | EntityTexture::MarkingsWhiteDots
            | EntityTexture::MarkingsBlackDots
            | EntityTexture::Donkey => Vec2::new(128.0, 128.0),
            _ => Vec2::new(64.0, 32.0),
        }
    }
}

pub const TEXTURES: usize = 32;

pub type MobVertices = [Vec<f32>; TEXTURES];

struct BoxPart {
    sheet: EntityTexture,
    origin: Vec3,
    size: Vec3,
    grow: f32,
    texture: Vec2,
    pivot: Vec3,
    rotation: Mat3,
    mirror: bool,
}

fn part(
    sheet: EntityTexture,
    texture: (f32, f32),
    origin: [f32; 3],
    size: [f32; 3],
    pivot: [f32; 3],
    rotation: Mat3,
) -> BoxPart {
    BoxPart {
        sheet,
        origin: Vec3::from(origin),
        size: Vec3::from(size),
        grow: 0.0,
        texture: Vec2::new(texture.0, texture.1),
        pivot: Vec3::from(pivot),
        rotation,
        mirror: false,
    }
}

fn mirrored(mut part: BoxPart) -> BoxPart {
    part.mirror = true;
    part
}

fn head_rotation(instance: &MobInstance) -> Mat3 {
    Mat3::from_rotation_y(instance.head_yaw) * Mat3::from_rotation_x(-instance.head_pitch)
}

fn leg_rotation(instance: &MobInstance, phase: f32) -> Mat3 {
    Mat3::from_rotation_x(
        (instance.limb_phase * LEG_FREQUENCY + phase).cos() * LEG_SWING * instance.limb_swing,
    )
}

mod equine;
mod small;

const WILD_TAIL_ANGLE: f32 = PI / 5.0;
const MUSHROOM_YAW: f32 = 42.0 * PI / 180.0;
const HEAD_MUSHROOM_YAW: f32 = 12.0 * PI / 180.0;
const CROSS_DIAGONALS: [[Vec2; 2]; 2] = [
    [Vec2::new(-0.45, -0.45), Vec2::new(0.45, 0.45)],
    [Vec2::new(-0.45, 0.45), Vec2::new(0.45, -0.45)],
];
const LEG_PHASES: [f32; 4] = [0.0, PI, PI, 0.0];

fn pig_parts(instance: &MobInstance) -> Vec<BoxPart> {
    let sheet = EntityTexture::Pig;
    let head = head_rotation(instance);
    let head_pivot = [0.0, 12.0, -6.0];
    let mut parts = vec![
        part(
            sheet,
            (0.0, 0.0),
            [-4.0, -4.0, -8.0],
            [8.0, 8.0, 8.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (16.0, 16.0),
            [-2.0, 0.0, -9.0],
            [4.0, 3.0, 1.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (28.0, 8.0),
            [-5.0, -10.0, -7.0],
            [10.0, 16.0, 8.0],
            [0.0, 11.0, 2.0],
            LAID_FLAT,
        ),
    ];
    let legs = [(-3.0, 7.0), (3.0, 7.0), (-3.0, -5.0), (3.0, -5.0)];
    for ((x, z), phase) in legs.into_iter().zip(LEG_PHASES) {
        parts.push(part(
            sheet,
            (0.0, 16.0),
            [-2.0, 0.0, -2.0],
            [4.0, 6.0, 4.0],
            [x, 18.0, z],
            leg_rotation(instance, phase),
        ));
    }
    parts
}

fn cow_parts(instance: &MobInstance, sheet: EntityTexture) -> Vec<BoxPart> {
    let head = head_rotation(instance);
    let head_pivot = [0.0, 4.0, -8.0];
    let body_pivot = [0.0, 5.0, 2.0];
    let mut parts = vec![
        part(
            sheet,
            (0.0, 0.0),
            [-4.0, -4.0, -6.0],
            [8.0, 8.0, 6.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (22.0, 0.0),
            [-5.0, -5.0, -4.0],
            [1.0, 3.0, 1.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (22.0, 0.0),
            [4.0, -5.0, -4.0],
            [1.0, 3.0, 1.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (18.0, 4.0),
            [-6.0, -10.0, -7.0],
            [12.0, 18.0, 10.0],
            body_pivot,
            LAID_FLAT,
        ),
        part(
            sheet,
            (52.0, 0.0),
            [-2.0, 2.0, -8.0],
            [4.0, 6.0, 1.0],
            body_pivot,
            LAID_FLAT,
        ),
    ];
    let legs = [(-4.0, 7.0), (4.0, 7.0), (-4.0, -6.0), (4.0, -6.0)];
    for ((x, z), phase) in legs.into_iter().zip(LEG_PHASES) {
        parts.push(part(
            sheet,
            (0.0, 16.0),
            [-2.0, 0.0, -2.0],
            [4.0, 12.0, 4.0],
            [x, 12.0, z],
            leg_rotation(instance, phase),
        ));
    }
    parts
}

fn sheep_parts(instance: &MobInstance) -> Vec<BoxPart> {
    let sheet = EntityTexture::Sheep;
    let fur = EntityTexture::SheepFur;
    let head = head_rotation(instance);
    let head_pivot = [0.0, 6.0, -8.0];
    let body_pivot = [0.0, 5.0, 2.0];
    let grown = |mut part: BoxPart, grow: f32| {
        part.grow = grow;
        part
    };
    let mut parts = vec![
        part(
            sheet,
            (0.0, 0.0),
            [-3.0, -4.0, -6.0],
            [6.0, 6.0, 8.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (28.0, 8.0),
            [-4.0, -10.0, -7.0],
            [8.0, 16.0, 6.0],
            body_pivot,
            LAID_FLAT,
        ),
        grown(
            part(
                fur,
                (0.0, 0.0),
                [-3.0, -4.0, -4.0],
                [6.0, 6.0, 6.0],
                head_pivot,
                head,
            ),
            0.6,
        ),
        grown(
            part(
                fur,
                (28.0, 8.0),
                [-4.0, -10.0, -7.0],
                [8.0, 16.0, 6.0],
                body_pivot,
                LAID_FLAT,
            ),
            1.75,
        ),
    ];
    let legs = [(-3.0, 7.0), (3.0, 7.0), (-3.0, -5.0), (3.0, -5.0)];
    for ((x, z), phase) in legs.into_iter().zip(LEG_PHASES) {
        let rotation = leg_rotation(instance, phase);
        parts.push(part(
            sheet,
            (0.0, 16.0),
            [-2.0, 0.0, -2.0],
            [4.0, 12.0, 4.0],
            [x, 12.0, z],
            rotation,
        ));
        parts.push(grown(
            part(
                fur,
                (0.0, 16.0),
                [-2.0, 0.0, -2.0],
                [4.0, 6.0, 4.0],
                [x, 12.0, z],
                rotation,
            ),
            0.5,
        ));
    }
    parts
}

fn chicken_parts(instance: &MobInstance) -> Vec<BoxPart> {
    let sheet = EntityTexture::Chicken;
    let head = head_rotation(instance);
    let head_pivot = [0.0, 15.0, -4.0];
    let flap = ((instance.flap_phase.sin() + 1.0) * instance.flap_amount).max(0.0);
    vec![
        part(
            sheet,
            (0.0, 0.0),
            [-2.0, -6.0, -2.0],
            [4.0, 6.0, 3.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (14.0, 0.0),
            [-2.0, -4.0, -4.0],
            [4.0, 2.0, 2.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (14.0, 4.0),
            [-1.0, -2.0, -3.0],
            [2.0, 2.0, 2.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (0.0, 9.0),
            [-3.0, -4.0, -3.0],
            [6.0, 8.0, 6.0],
            [0.0, 16.0, 0.0],
            LAID_FLAT,
        ),
        part(
            sheet,
            (26.0, 0.0),
            [-1.0, 0.0, -3.0],
            [3.0, 5.0, 3.0],
            [-2.0, 19.0, 1.0],
            leg_rotation(instance, 0.0),
        ),
        part(
            sheet,
            (26.0, 0.0),
            [-1.0, 0.0, -3.0],
            [3.0, 5.0, 3.0],
            [1.0, 19.0, 1.0],
            leg_rotation(instance, PI),
        ),
        part(
            sheet,
            (24.0, 13.0),
            [0.0, 0.0, -3.0],
            [1.0, 4.0, 6.0],
            [-4.0, 13.0, 0.0],
            Mat3::from_rotation_z(flap),
        ),
        part(
            sheet,
            (24.0, 13.0),
            [-1.0, 0.0, -3.0],
            [1.0, 4.0, 6.0],
            [4.0, 13.0, 0.0],
            Mat3::from_rotation_z(-flap),
        ),
    ]
}

fn polar_bear_parts(instance: &MobInstance) -> Vec<BoxPart> {
    let sheet = EntityTexture::PolarBear;
    let head = head_rotation(instance);
    let head_pivot = [0.0, 10.0, -16.0];
    let mut parts = vec![
        part(
            sheet,
            (0.0, 0.0),
            [-3.5, -3.0, -3.0],
            [7.0, 7.0, 7.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (0.0, 44.0),
            [-2.5, 1.0, -6.0],
            [5.0, 3.0, 3.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (26.0, 0.0),
            [-4.5, -4.0, -1.0],
            [2.0, 2.0, 1.0],
            head_pivot,
            head,
        ),
        mirrored(part(
            sheet,
            (26.0, 0.0),
            [2.5, -4.0, -1.0],
            [2.0, 2.0, 1.0],
            head_pivot,
            head,
        )),
        part(
            sheet,
            (0.0, 19.0),
            [-5.0, -13.0, -7.0],
            [14.0, 14.0, 11.0],
            [-2.0, 9.0, 12.0],
            LAID_FLAT,
        ),
        part(
            sheet,
            (39.0, 0.0),
            [-4.0, -25.0, -7.0],
            [12.0, 12.0, 10.0],
            [-2.0, 9.0, 12.0],
            LAID_FLAT,
        ),
    ];
    let legs = [
        (-3.5, 6.0, (50.0, 22.0), 8.0),
        (3.5, 6.0, (50.0, 22.0), 8.0),
        (-2.5, -7.0, (50.0, 40.0), 6.0),
        (2.5, -7.0, (50.0, 40.0), 6.0),
    ];
    for ((x, z, texture, depth), phase) in legs.into_iter().zip(LEG_PHASES) {
        parts.push(part(
            sheet,
            texture,
            [-2.0, 0.0, -2.0],
            [4.0, 10.0, depth],
            [x, 14.0, z],
            leg_rotation(instance, phase),
        ));
    }
    parts
}

fn wolf_parts(instance: &MobInstance) -> Vec<BoxPart> {
    let sheet = EntityTexture::Wolf;
    let head = head_rotation(instance);
    let head_pivot = [-1.0, 13.5, -7.0];
    let mut parts = vec![
        part(
            sheet,
            (0.0, 0.0),
            [-2.0, -3.0, -2.0],
            [6.0, 6.0, 4.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (16.0, 14.0),
            [-2.0, -5.0, 0.0],
            [2.0, 2.0, 1.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (16.0, 14.0),
            [2.0, -5.0, 0.0],
            [2.0, 2.0, 1.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (0.0, 10.0),
            [-0.5, 0.0, -5.0],
            [3.0, 3.0, 4.0],
            head_pivot,
            head,
        ),
        part(
            sheet,
            (18.0, 14.0),
            [-3.0, -2.0, -3.0],
            [6.0, 9.0, 6.0],
            [0.0, 14.0, 2.0],
            LAID_FLAT,
        ),
        part(
            sheet,
            (21.0, 0.0),
            [-3.0, -3.0, -3.0],
            [8.0, 6.0, 7.0],
            [-1.0, 14.0, -3.0],
            LAID_FLAT,
        ),
        part(
            sheet,
            (9.0, 18.0),
            [0.0, 0.0, -1.0],
            [2.0, 8.0, 2.0],
            [-1.0, 12.0, 8.0],
            Mat3::from_rotation_x(WILD_TAIL_ANGLE),
        ),
    ];
    let legs = [(-2.5, 7.0), (0.5, 7.0), (-2.5, -4.0), (0.5, -4.0)];
    for ((x, z), phase) in legs.into_iter().zip(LEG_PHASES) {
        parts.push(part(
            sheet,
            (0.0, 18.0),
            [0.0, 0.0, -1.0],
            [2.0, 8.0, 2.0],
            [x, 16.0, z],
            leg_rotation(instance, phase),
        ));
    }
    parts
}

fn mushroom_frame(position: Vec3, yaw: f32) -> (Vec3, Mat3) {
    (position, Mat3::from_rotation_y(yaw))
}

fn push_mushrooms(instance: &MobInstance, out: &mut Vec<f32>) {
    let back = mushroom_frame(Vec3::new(0.2, 0.35, 0.5), MUSHROOM_YAW);
    let second = (
        back.0 + back.1 * Vec3::new(0.1, 0.0, -0.6),
        Mat3::from_rotation_y(2.0 * MUSHROOM_YAW),
    );
    let head = head_rotation(instance);
    let head_pivot = Vec3::new(0.0, 4.0, -8.0);
    let on_head = mushroom_frame(Vec3::new(0.0, 0.7, -0.2), HEAD_MUSHROOM_YAW);
    for (frame, rides_head) in [(back, false), (second, false), (on_head, true)] {
        for corner in CROSS_DIAGONALS {
            let point = |x: f32, y: f32, z: f32| {
                let block = frame.0 + frame.1 * Vec3::new(x, y, z);
                let model = Vec3::new(block.x, -block.y, block.z) * PIXELS_PER_BLOCK;
                if rides_head {
                    head_pivot + head * model
                } else {
                    model
                }
            };
            let (a, b) = (corner[0], corner[1]);
            let quad = [
                (point(a.x, -0.5, a.y), Vec2::new(0.0, 16.0)),
                (point(b.x, -0.5, b.y), Vec2::new(16.0, 16.0)),
                (point(b.x, 0.5, b.y), Vec2::new(16.0, 0.0)),
                (point(a.x, 0.5, a.y), Vec2::new(0.0, 0.0)),
            ];
            push_double_sided(quad, instance, out);
        }
    }
}

fn push_double_sided(quad: [(Vec3, Vec2); 4], instance: &MobInstance, out: &mut Vec<f32>) {
    let world = quad.map(|(p, uv)| (to_world(p, instance), uv / MUSHROOM_UNITS));
    let normal = (world[1].0 - world[0].0)
        .cross(world[3].0 - world[0].0)
        .normalize_or_zero();
    let shade = face_shade(normal);
    let reversed = [world[1], world[0], world[3], world[2]];
    for side in [world, reversed] {
        for index in [0, 1, 2, 0, 2, 3] {
            let (position, uv) = side[index];
            out.extend_from_slice(&[
                position.x,
                position.y,
                position.z,
                uv.x,
                uv.y,
                shade,
                instance.sky_level,
                instance.block_level,
            ]);
        }
    }
}

fn to_world(model: Vec3, instance: &MobInstance) -> Vec3 {
    let scale = match instance.kind {
        ModelKind::Donkey => equine::DONKEY_SCALE,
        ModelKind::Rabbit => small::RABBIT_SCALE,
        _ => 1.0,
    };
    let local = Vec3::new(-model.x, MODEL_FEET - model.y, model.z) * scale / PIXELS_PER_BLOCK;
    instance.feet + Mat3::from_rotation_y(-instance.yaw - FRAC_PI_2) * local
}

fn face_shade(normal: Vec3) -> f32 {
    normal.y.max(0.0)
        + 0.5 * (-normal.y).max(0.0)
        + 0.6 * normal.x * normal.x
        + 0.8 * normal.z * normal.z
}

pub fn push_mob(instance: &MobInstance, out: &mut MobVertices) {
    let parts = match instance.kind {
        ModelKind::Pig => pig_parts(instance),
        ModelKind::Chicken => chicken_parts(instance),
        ModelKind::Cow => cow_parts(instance, EntityTexture::Cow),
        ModelKind::Mooshroom => cow_parts(instance, EntityTexture::Mooshroom),
        ModelKind::PolarBear => polar_bear_parts(instance),
        ModelKind::Wolf => wolf_parts(instance),
        ModelKind::Sheep => sheep_parts(instance),
        ModelKind::Horse => equine::horse_parts(instance),
        ModelKind::Donkey => equine::donkey_parts(instance),
        ModelKind::Llama => equine::llama_parts(instance),
        ModelKind::Rabbit => small::rabbit_parts(instance),
        ModelKind::Ocelot => small::ocelot_parts(instance),
    };
    for part in &parts {
        push_box(part, instance, &mut out[part.sheet as usize]);
    }
    if instance.kind == ModelKind::Mooshroom {
        push_mushrooms(instance, &mut out[EntityTexture::Mushroom as usize]);
    }
}

fn push_box(part: &BoxPart, instance: &MobInstance, out: &mut Vec<f32>) {
    let (lo, hi) = (
        part.origin - Vec3::splat(part.grow),
        part.origin + part.size + Vec3::splat(part.grow),
    );
    let (dx, dy, dz) = (part.size.x, part.size.y, part.size.z);
    let (u, v) = (part.texture.x, part.texture.y);
    let corner = |x: f32, y: f32, z: f32| {
        to_world(part.pivot + part.rotation * Vec3::new(x, y, z), instance)
    };
    let center = corner(
        (lo.x + hi.x) / 2.0,
        (lo.y + hi.y) / 2.0,
        (lo.z + hi.z) / 2.0,
    );
    let faces: [[(Vec3, Vec2); 4]; 6] = [
        [
            (Vec3::new(lo.x, lo.y, lo.z), Vec2::new(u + dz, v + dz)),
            (Vec3::new(hi.x, lo.y, lo.z), Vec2::new(u + dz + dx, v + dz)),
            (
                Vec3::new(hi.x, hi.y, lo.z),
                Vec2::new(u + dz + dx, v + dz + dy),
            ),
            (Vec3::new(lo.x, hi.y, lo.z), Vec2::new(u + dz, v + dz + dy)),
        ],
        [
            (
                Vec3::new(hi.x, lo.y, hi.z),
                Vec2::new(u + 2.0 * dz + dx, v + dz),
            ),
            (
                Vec3::new(lo.x, lo.y, hi.z),
                Vec2::new(u + 2.0 * dz + 2.0 * dx, v + dz),
            ),
            (
                Vec3::new(lo.x, hi.y, hi.z),
                Vec2::new(u + 2.0 * dz + 2.0 * dx, v + dz + dy),
            ),
            (
                Vec3::new(hi.x, hi.y, hi.z),
                Vec2::new(u + 2.0 * dz + dx, v + dz + dy),
            ),
        ],
        [
            (Vec3::new(hi.x, lo.y, lo.z), Vec2::new(u + dz + dx, v + dz)),
            (
                Vec3::new(hi.x, lo.y, hi.z),
                Vec2::new(u + 2.0 * dz + dx, v + dz),
            ),
            (
                Vec3::new(hi.x, hi.y, hi.z),
                Vec2::new(u + 2.0 * dz + dx, v + dz + dy),
            ),
            (
                Vec3::new(hi.x, hi.y, lo.z),
                Vec2::new(u + dz + dx, v + dz + dy),
            ),
        ],
        [
            (Vec3::new(lo.x, lo.y, hi.z), Vec2::new(u, v + dz)),
            (Vec3::new(lo.x, lo.y, lo.z), Vec2::new(u + dz, v + dz)),
            (Vec3::new(lo.x, hi.y, lo.z), Vec2::new(u + dz, v + dz + dy)),
            (Vec3::new(lo.x, hi.y, hi.z), Vec2::new(u, v + dz + dy)),
        ],
        [
            (Vec3::new(lo.x, lo.y, hi.z), Vec2::new(u + dz, v)),
            (Vec3::new(hi.x, lo.y, hi.z), Vec2::new(u + dz + dx, v)),
            (Vec3::new(hi.x, lo.y, lo.z), Vec2::new(u + dz + dx, v + dz)),
            (Vec3::new(lo.x, lo.y, lo.z), Vec2::new(u + dz, v + dz)),
        ],
        [
            (Vec3::new(hi.x, hi.y, hi.z), Vec2::new(u + dz + 2.0 * dx, v)),
            (Vec3::new(lo.x, hi.y, hi.z), Vec2::new(u + dz + dx, v)),
            (Vec3::new(lo.x, hi.y, lo.z), Vec2::new(u + dz + dx, v + dz)),
            (
                Vec3::new(hi.x, hi.y, lo.z),
                Vec2::new(u + dz + 2.0 * dx, v + dz),
            ),
        ],
    ];
    let units = part.sheet.units();
    let regions = [
        (u + dz, u + dz + dx, 0.0),
        (u + 2.0 * dz + dx, u + 2.0 * dz + 2.0 * dx, 0.0),
        (u + dz + dx, u + 2.0 * dz + dx, -(dz + dx)),
        (u, u + dz, dz + dx),
        (u + dz, u + dz + dx, 0.0),
        (u + dz + dx, u + dz + 2.0 * dx, 0.0),
    ];
    for (index, face) in faces.into_iter().enumerate() {
        let flip = |uv: Vec2| {
            let (from, to, shift) = regions[index];
            if part.mirror {
                Vec2::new(from + to - uv.x + shift, uv.y)
            } else {
                uv
            }
        };
        let mut quad = face.map(|(p, uv)| (corner(p.x, p.y, p.z), flip(uv) / units));
        let face_center = quad.iter().map(|(p, _)| *p).sum::<Vec3>() / 4.0;
        let normal = (quad[1].0 - quad[0].0).cross(quad[3].0 - quad[0].0);
        if normal.dot(face_center - center) < 0.0 {
            quad.swap(1, 3);
        }
        let normal = (quad[1].0 - quad[0].0)
            .cross(quad[3].0 - quad[0].0)
            .normalize_or_zero();
        let shade = face_shade(normal);
        for index in [0, 1, 2, 0, 2, 3] {
            let (position, uv) = quad[index];
            out.extend_from_slice(&[
                position.x,
                position.y,
                position.z,
                uv.x,
                uv.y,
                shade,
                instance.sky_level,
                instance.block_level,
            ]);
        }
    }
}

const BLOB_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32];

pub fn blob_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: BLOB_BYTES as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &BLOB_ATTRIBUTES,
    }
}

pub fn push_blobs(instance: &MobInstance, out: &mut Vec<f32>) {
    let center = Vec2::new(instance.feet.x, instance.feet.z);
    for patch in &instance.shadow {
        let corner = |x: f32, z: f32| {
            let local = (Vec2::new(x, z) - center) / instance.shadow_radius;
            [x, patch.y, z, local.x, local.y, patch.opacity]
        };
        let quad = [
            corner(patch.min.x, patch.min.y),
            corner(patch.min.x, patch.max.y),
            corner(patch.max.x, patch.max.y),
            corner(patch.max.x, patch.min.y),
        ];
        for index in [0, 1, 2, 0, 2, 3] {
            out.extend_from_slice(&quad[index]);
        }
    }
}

pub fn visible_blobs(instances: &[MobInstance], visible: impl Fn(Vec3, Vec3) -> bool) -> Vec<f32> {
    let mut out = Vec::new();
    for instance in instances {
        let (min, max) = instance.bounds();
        if visible(min, max) {
            push_blobs(instance, &mut out);
        }
    }
    out
}

pub fn visible_vertices(
    instances: &[MobInstance],
    visible: impl Fn(Vec3, Vec3) -> bool,
) -> MobVertices {
    let mut out = MobVertices::default();
    for instance in instances {
        let (min, max) = instance.bounds();
        if visible(min, max) {
            push_mob(instance, &mut out);
        }
    }
    out
}

pub fn texture_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("entity"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

pub fn create_texture_groups(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    images: &EntityImages,
) -> Vec<wgpu::BindGroup> {
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("entity"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    EntityTexture::ALL
        .into_iter()
        .map(|sheet| {
            let image = match sheet {
                EntityTexture::Pig => &images.pig,
                EntityTexture::Chicken => &images.chicken,
                EntityTexture::Cow => &images.cow,
                EntityTexture::Sheep => &images.sheep,
                EntityTexture::SheepFur => &images.sheep_fur,
                EntityTexture::Mooshroom => &images.mooshroom,
                EntityTexture::PolarBear => &images.polar_bear,
                EntityTexture::Wolf => &images.wolf,
                EntityTexture::Mushroom => &images.mushroom_red,
                EntityTexture::HorseWhite => &images.horse[0],
                EntityTexture::HorseCreamy => &images.horse[1],
                EntityTexture::HorseChestnut => &images.horse[2],
                EntityTexture::HorseBrown => &images.horse[3],
                EntityTexture::HorseBlack => &images.horse[4],
                EntityTexture::HorseGray => &images.horse[5],
                EntityTexture::HorseDarkBrown => &images.horse[6],
                EntityTexture::MarkingsWhite => &images.horse_markings[0],
                EntityTexture::MarkingsWhiteField => &images.horse_markings[1],
                EntityTexture::MarkingsWhiteDots => &images.horse_markings[2],
                EntityTexture::MarkingsBlackDots => &images.horse_markings[3],
                EntityTexture::Donkey => &images.donkey,
                EntityTexture::LlamaCreamy => &images.llama[0],
                EntityTexture::LlamaWhite => &images.llama[1],
                EntityTexture::LlamaBrown => &images.llama[2],
                EntityTexture::LlamaGray => &images.llama[3],
                EntityTexture::RabbitBrown => &images.rabbit[0],
                EntityTexture::RabbitWhite => &images.rabbit[1],
                EntityTexture::RabbitBlack => &images.rabbit[2],
                EntityTexture::RabbitSplotched => &images.rabbit[3],
                EntityTexture::RabbitGold => &images.rabbit[4],
                EntityTexture::RabbitSalt => &images.rabbit[5],
                EntityTexture::Ocelot => &images.ocelot,
            };
            let texture = device.create_texture_with_data(
                queue,
                &wgpu::TextureDescriptor {
                    label: Some("entity sheet"),
                    size: wgpu::Extent3d {
                        width: image.width,
                        height: image.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                &image.rgba,
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("entity"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
