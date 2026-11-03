use std::f32::consts::FRAC_PI_2;

use glam::{Mat3, Vec2, Vec3};
use wgpu::util::DeviceExt;
use world::pack::EntityImages;

pub const VERTEX_FLOATS: usize = 8;
pub const VERTEX_BYTES: usize = VERTEX_FLOATS * 4;
pub const BLOB_FLOATS: usize = 6;
pub const BLOB_BYTES: usize = BLOB_FLOATS * 4;
const SHEET_UNITS: Vec2 = Vec2::new(64.0, 32.0);
const PIXELS_PER_BLOCK: f32 = 16.0;
const MODEL_FEET: f32 = 24.0;
const CULL_RADIUS: f32 = 1.2;
const SHADOW_DROP: f32 = 2.0;
const CULL_HEIGHT: f32 = 1.4;
const LEG_SWING: f32 = 1.4;
const LEG_FREQUENCY: f32 = 0.6662;

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
    pub feet: Vec3,
    pub yaw: f32,
    pub head_yaw: f32,
    pub head_pitch: f32,
    pub limb_phase: f32,
    pub limb_swing: f32,
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

struct BoxPart {
    origin: Vec3,
    size: Vec3,
    texture: Vec2,
    pivot: Vec3,
    rotation: Mat3,
}

fn pig_parts(instance: &MobInstance) -> [BoxPart; 7] {
    let head_rotation =
        Mat3::from_rotation_y(instance.head_yaw) * Mat3::from_rotation_x(-instance.head_pitch);
    let head_pivot = Vec3::new(0.0, 12.0, -6.0);
    let leg = |x: f32, z: f32, phase: f32| BoxPart {
        origin: Vec3::new(-2.0, 0.0, -2.0),
        size: Vec3::new(4.0, 6.0, 4.0),
        texture: Vec2::new(0.0, 16.0),
        pivot: Vec3::new(x, 18.0, z),
        rotation: Mat3::from_rotation_x(
            (instance.limb_phase * LEG_FREQUENCY + phase).cos() * LEG_SWING * instance.limb_swing,
        ),
    };
    let pi = std::f32::consts::PI;
    [
        BoxPart {
            origin: Vec3::new(-4.0, -4.0, -8.0),
            size: Vec3::new(8.0, 8.0, 8.0),
            texture: Vec2::new(0.0, 0.0),
            pivot: head_pivot,
            rotation: head_rotation,
        },
        BoxPart {
            origin: Vec3::new(-2.0, 0.0, -9.0),
            size: Vec3::new(4.0, 3.0, 1.0),
            texture: Vec2::new(16.0, 16.0),
            pivot: head_pivot,
            rotation: head_rotation,
        },
        BoxPart {
            origin: Vec3::new(-5.0, -10.0, -7.0),
            size: Vec3::new(10.0, 16.0, 8.0),
            texture: Vec2::new(28.0, 8.0),
            pivot: Vec3::new(0.0, 11.0, 2.0),
            rotation: Mat3::from_rotation_x(FRAC_PI_2),
        },
        leg(-3.0, 7.0, 0.0),
        leg(3.0, 7.0, pi),
        leg(-3.0, -5.0, pi),
        leg(3.0, -5.0, 0.0),
    ]
}

fn to_world(model: Vec3, instance: &MobInstance) -> Vec3 {
    let local = Vec3::new(-model.x, MODEL_FEET - model.y, model.z) / PIXELS_PER_BLOCK;
    instance.feet + Mat3::from_rotation_y(-instance.yaw - FRAC_PI_2) * local
}

fn face_shade(normal: Vec3) -> f32 {
    normal.y.max(0.0)
        + 0.5 * (-normal.y).max(0.0)
        + 0.6 * normal.x * normal.x
        + 0.8 * normal.z * normal.z
}

pub fn push_mob(instance: &MobInstance, out: &mut Vec<f32>) {
    let parts = match instance.kind {
        ModelKind::Pig => pig_parts(instance),
    };
    for part in &parts {
        push_box(part, instance, out);
    }
}

fn push_box(part: &BoxPart, instance: &MobInstance, out: &mut Vec<f32>) {
    let (lo, hi) = (part.origin, part.origin + part.size);
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
    for face in faces {
        let mut quad = face.map(|(p, uv)| (corner(p.x, p.y, p.z), uv / SHEET_UNITS));
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
) -> Vec<f32> {
    let mut out = Vec::new();
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

pub fn create_texture_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    images: &EntityImages,
) -> wgpu::BindGroup {
    let pig = &images.pig;
    let texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("entity pig"),
            size: wgpu::Extent3d {
                width: pig.width,
                height: pig.height,
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
        &pig.rgba,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("entity"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
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
}

#[cfg(test)]
mod tests;
