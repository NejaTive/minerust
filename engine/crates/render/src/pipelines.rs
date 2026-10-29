use crate::layout::slot_layout;
use crate::outline::OUTLINE_VERTEX_SIZE;
use crate::shaders;

pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockPass {
    /// Непрозрачные блоки и листва: с записью глубины, листва — через `discard`.
    Solid,
    Water,
}

/// Пайплайн граней блоков. У воды смешение по альфе, глубина проверяется, но не
/// пишется (вода не заслоняет воду за собой), и обе стороны граней: поверхность видна
/// и из-под воды.
pub(crate) fn create_block_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    pass: BlockPass,
) -> wgpu::RenderPipeline {
    let water = pass == BlockPass::Water;
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(if water { "water" } else { "main" }),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(slot_layout())],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(if water { "fs_water" } else { "fs_main" }),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: water.then_some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: (!water).then_some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(!water),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// Пайплайн рамки вокруг блока под прицелом: линии поверх рельефа с тем же буфером глубины.
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
        // Рамка раздута над гранями, так что `LessEqual` показывает рёбра на видимых
        // гранях и прячет те, что за блоком. Без записи: рамка не должна заслонять ничего.
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

/// Пайплайн неба: треугольник на дальней плоскости без буферов. `LessEqual` без записи
/// глубины оставляет рельеф и рамку нетронутыми.
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
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}
