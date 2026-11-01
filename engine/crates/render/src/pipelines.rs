use crate::layout::slot_layout;
use crate::outline::OUTLINE_VERTEX_SIZE;
use crate::shaders;

pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub(crate) const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub(crate) struct SceneLayouts {
    pub plain: wgpu::PipelineLayout,
    pub blocks: wgpu::PipelineLayout,
}

pub(crate) struct ScenePipelines {
    pub solid: wgpu::RenderPipeline,
    pub cutout: wgpu::RenderPipeline,
    pub water: wgpu::RenderPipeline,
    pub ice: wgpu::RenderPipeline,
    pub ice_depth: wgpu::RenderPipeline,
    pub outline: wgpu::RenderPipeline,
    pub sky: wgpu::RenderPipeline,
    pub cloud: wgpu::RenderPipeline,
    pub rain: wgpu::RenderPipeline,
}

impl ScenePipelines {
    pub(crate) fn new(
        device: &wgpu::Device,
        layouts: &SceneLayouts,
        shader: &wgpu::ShaderModule,
        format: wgpu::TextureFormat,
    ) -> Self {
        let block = |pass| create_block_pipeline(device, &layouts.blocks, shader, format, pass);
        Self {
            solid: block(BlockPass::Solid),
            cutout: block(BlockPass::Cutout),
            water: block(BlockPass::Water),
            ice: block(BlockPass::Ice),
            ice_depth: block(BlockPass::IceDepth),
            outline: create_outline_pipeline(device, &layouts.plain, format),
            sky: create_sky_pipeline(device, &layouts.plain, format),
            cloud: create_cloud_pipeline(device, &layouts.plain, format),
            rain: create_rain_pipeline(device, &layouts.plain, format),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockPass {
    Solid,
    Cutout,
    Water,
    Ice,
    IceDepth,
}

pub(crate) fn create_block_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    pass: BlockPass,
) -> wgpu::RenderPipeline {
    let water = matches!(
        pass,
        BlockPass::Water | BlockPass::Ice | BlockPass::IceDepth
    );
    let ice = matches!(pass, BlockPass::Ice | BlockPass::IceDepth);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(match pass {
            BlockPass::Solid => "main",
            BlockPass::Cutout => "cutout",
            BlockPass::Water => "water",
            BlockPass::Ice => "ice",
            BlockPass::IceDepth => "ice depth",
        }),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(slot_layout())],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(match pass {
                BlockPass::Solid => "fs_main",
                BlockPass::Cutout => "fs_cutout",
                BlockPass::Water => "fs_water",
                BlockPass::Ice | BlockPass::IceDepth => "fs_ice",
            }),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: water.then_some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: if pass == BlockPass::IceDepth {
                    wgpu::ColorWrites::empty()
                } else {
                    wgpu::ColorWrites::ALL
                },
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: (!water || ice).then_some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(!water || pass == BlockPass::IceDepth),
            depth_compare: Some(if pass == BlockPass::Ice {
                wgpu::CompareFunction::LessEqual
            } else {
                wgpu::CompareFunction::Less
            }),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

pub(crate) fn create_outline_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = shaders::module(device, "outline", shaders::OUTLINE_SHADER);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("outline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: OUTLINE_VERTEX_SIZE as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x3],
            })],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(format.into())],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

pub(crate) fn create_sky_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = shaders::module(device, "sky", shaders::SKY_SHADER);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sky"),
        layout: Some(layout),
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
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

pub(crate) fn create_cloud_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = shaders::module(device, "clouds", shaders::CLOUDS_SHADER);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("clouds"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

pub(crate) fn create_rain_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = shaders::module(device, "rain", shaders::RAIN_SHADER);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("rain"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

pub(crate) fn create_depth_view(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}
