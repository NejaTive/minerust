use crate::bloom::{BloomTargets, create_bloom_layout, create_linear_sampler};
use glam::{Mat4, Vec2, Vec3};

use crate::pipelines::{DEPTH_FORMAT, HDR_FORMAT};
use crate::shaders;

const RAYS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;

pub(crate) struct PostLayouts {
    pub post: wgpu::BindGroupLayout,
    pub scene_copy: wgpu::BindGroupLayout,
    pub bloom: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
}

impl PostLayouts {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        Self {
            post: create_post_layout(device),
            scene_copy: create_scene_copy_layout(device),
            bloom: create_bloom_layout(device),
            sampler: create_linear_sampler(device),
        }
    }
}

pub(crate) struct HdrTargets {
    pub color_texture: wgpu::Texture,
    pub color: wgpu::TextureView,
    pub color_copy: wgpu::Texture,
    pub depth_copy: wgpu::Texture,
    pub scene_copy_group: wgpu::BindGroup,
    pub rays: wgpu::TextureView,
    pub post_group: wgpu::BindGroup,
    pub bloom: BloomTargets,
}

pub(crate) fn create_scene_copy_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scene copy"),
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
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn create_target(
    device: &wgpu::Device,
    label: &str,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
    width: u32,
    height: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

pub(crate) fn create_scene_copy_group(
    device: &wgpu::Device,
    layouts: &PostLayouts,
    color: &wgpu::Texture,
    depth: &wgpu::Texture,
) -> wgpu::BindGroup {
    let color = color.create_view(&wgpu::TextureViewDescriptor::default());
    let depth = depth.create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scene copy"),
        layout: &layouts.scene_copy,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&color),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&depth),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&layouts.sampler),
            },
        ],
    })
}

pub(crate) fn create_scene_copy_stub(
    device: &wgpu::Device,
    layouts: &PostLayouts,
) -> wgpu::BindGroup {
    let usage = wgpu::TextureUsages::TEXTURE_BINDING;
    let color = create_target(device, "scene copy stub", HDR_FORMAT, usage, 1, 1);
    let depth = create_target(device, "scene copy stub", DEPTH_FORMAT, usage, 1, 1);
    create_scene_copy_group(device, layouts, &color, &depth)
}

impl HdrTargets {
    pub(crate) fn new(
        device: &wgpu::Device,
        layouts: &PostLayouts,
        width: u32,
        height: u32,
    ) -> Self {
        let texture = create_target(
            device,
            "hdr color",
            HDR_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            width,
            height,
        );
        let color_copy = create_target(
            device,
            "hdr color copy",
            HDR_FORMAT,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            width,
            height,
        );
        let depth_copy = create_target(
            device,
            "depth copy",
            DEPTH_FORMAT,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            width,
            height,
        );
        let rays = create_target(
            device,
            "sun rays",
            RAYS_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            (width / 2).max(1),
            (height / 2).max(1),
        )
        .create_view(&wgpu::TextureViewDescriptor::default());
        let scene_copy_group = create_scene_copy_group(device, layouts, &color_copy, &depth_copy);
        let color = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let (bloom, bloom_result) = BloomTargets::new(
            device,
            &layouts.bloom,
            &layouts.sampler,
            &color,
            width,
            height,
        );
        let post_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post"),
            layout: &layouts.post,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&bloom_result),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&layouts.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&rays),
                },
            ],
        });
        Self {
            color_texture: texture,
            color,
            color_copy,
            depth_copy,
            scene_copy_group,
            rays,
            post_group,
            bloom,
        }
    }
}

pub(crate) fn create_post_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("post"),
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
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
        ],
    })
}

pub(crate) fn create_post_pipeline(
    device: &wgpu::Device,
    globals_layout: &wgpu::BindGroupLayout,
    post_layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = shaders::module(device, "post", shaders::POST_SHADER);
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("post"),
        bind_group_layouts: &[Some(globals_layout), Some(post_layout)],
        ..Default::default()
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("post"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(format.into())],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

pub(crate) fn create_rays_pipeline(
    device: &wgpu::Device,
    globals_layout: &wgpu::BindGroupLayout,
    scene_copy_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = shaders::module(device, "rays", shaders::RAYS_SHADER);
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("rays"),
        bind_group_layouts: &[Some(globals_layout), Some(scene_copy_layout)],
        ..Default::default()
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("rays"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(RAYS_FORMAT.into())],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Позиция источника света (солнце, ночью луна) в UV экрана и сила лучей: гаснут около
/// горизонта, в дождь, под водой и когда источник далеко за кадром. Лунные слабее солнечных.
pub(crate) fn light_rays(
    view_proj: Mat4,
    camera_pos: Vec3,
    sun_dir: Vec3,
    night: f32,
    rain: f32,
    underwater: bool,
) -> (Vec2, f32) {
    let sun_up = sun_dir.y >= 0.0;
    let dir = if sun_up { sun_dir } else { -sun_dir };
    let clip = view_proj * (camera_pos + dir.normalize() * SUN_DISTANCE).extend(1.0);
    if clip.w <= 0.0 || underwater {
        return (Vec2::ZERO, 0.0);
    }
    let ndc = Vec2::new(clip.x, clip.y) / clip.w;
    let uv = Vec2::new(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    let on_screen = 1.0 - smoothstep(1.0, 1.8, ndc.abs().max_element());
    let elevation = smoothstep(0.0, 0.25, dir.y);
    let source = if sun_up {
        1.0 - night
    } else {
        MOON_RAY_GAIN * night
    };
    (uv, on_screen * elevation * source * (1.0 - rain))
}

const MOON_RAY_GAIN: f32 = 0.3;
const SUN_DISTANCE: f32 = 100.0;

#[cfg(test)]
mod tests;
