use crate::pipelines::HDR_FORMAT;
use crate::shaders;

const MAX_MIPS: u32 = 6;

pub(crate) struct BloomPipelines {
    first: wgpu::RenderPipeline,
    down: wgpu::RenderPipeline,
    up: wgpu::RenderPipeline,
}

pub(crate) struct BloomTargets {
    views: Vec<wgpu::TextureView>,
    down_groups: Vec<wgpu::BindGroup>,
    up_groups: Vec<wgpu::BindGroup>,
}

pub(crate) fn create_bloom_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bloom"),
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

pub(crate) fn create_linear_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("linear clamp"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    })
}

impl BloomPipelines {
    pub(crate) fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let shader = shaders::module(device, "bloom", shaders::BLOOM_SHADER);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom"),
            bind_group_layouts: &[Some(layout)],
            ..Default::default()
        });
        let make = |entry: &str, blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: HDR_FORMAT,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let additive = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        Self {
            first: make("fs_down_first", None),
            down: make("fs_down", None),
            up: make(
                "fs_up",
                Some(wgpu::BlendState {
                    color: additive,
                    alpha: additive,
                }),
            ),
        }
    }
}

impl BloomTargets {
    pub(crate) fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        scene: &wgpu::TextureView,
        width: u32,
        height: u32,
    ) -> (Self, wgpu::TextureView) {
        let (width, height) = ((width / 2).max(1), (height / 2).max(1));
        let mips = MAX_MIPS.min(width.min(height).ilog2() + 1);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bloom"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: mips,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: HDR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let views: Vec<_> = (0..mips)
            .map(|mip| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    base_mip_level: mip,
                    mip_level_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        let group = |source: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bloom"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            })
        };
        let down_groups = std::iter::once(group(scene))
            .chain(views[..views.len() - 1].iter().map(group))
            .collect();
        let up_groups = views[1..].iter().map(group).collect();
        let result = views[0].clone();
        (
            Self {
                views,
                down_groups,
                up_groups,
            },
            result,
        )
    }

    pub(crate) fn encode(&self, encoder: &mut wgpu::CommandEncoder, pipelines: &BloomPipelines) {
        let draw = |encoder: &mut wgpu::CommandEncoder,
                    view: &wgpu::TextureView,
                    load: wgpu::LoadOp<wgpu::Color>,
                    pipeline: &wgpu::RenderPipeline,
                    group: &wgpu::BindGroup| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.draw(0..3, 0..1);
        };
        let clear = wgpu::LoadOp::Clear(wgpu::Color::BLACK);
        for (mip, (view, group)) in self.views.iter().zip(&self.down_groups).enumerate() {
            let pipeline = if mip == 0 {
                &pipelines.first
            } else {
                &pipelines.down
            };
            draw(encoder, view, clear, pipeline, group);
        }
        for (view, group) in self.views.iter().zip(&self.up_groups).rev() {
            draw(encoder, view, wgpu::LoadOp::Load, &pipelines.up, group);
        }
    }
}
