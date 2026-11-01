use crate::bloom::{BloomTargets, create_bloom_layout, create_linear_sampler};
use crate::pipelines::{DEPTH_FORMAT, HDR_FORMAT};
use crate::shaders;

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
            ],
        });
        Self {
            color_texture: texture,
            color,
            color_copy,
            depth_copy,
            scene_copy_group,
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
