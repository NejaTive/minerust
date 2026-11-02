use crate::exposure::FIXED_EXPOSURE;
use crate::shaders;

const GRID_SIZE: u32 = 64;
const BLOCK_GRID_SIZE: u32 = 8;
const LUMA_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;
pub(crate) const EXPOSURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;
const PARAMS_SIZE: u64 = 16;

pub(crate) struct ExposureState {
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    sample: wgpu::RenderPipeline,
    reduce: wgpu::RenderPipeline,
    adapt: wgpu::RenderPipeline,
    grid_view: wgpu::TextureView,
    block_view: wgpu::TextureView,
    reduce_group: wgpu::BindGroup,
    adapt_group: wgpu::BindGroup,
    current: wgpu::Texture,
    previous: wgpu::Texture,
    previous_view: wgpu::TextureView,
    params: wgpu::Buffer,
    pub view: wgpu::TextureView,
}

fn texture(
    device: &wgpu::Device,
    label: &str,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
    size: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size,
            height: size,
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

fn layout_entry(binding: u32, ty: wgpu::BindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty,
        count: None,
    }
}

pub(crate) fn float_texture(filterable: bool) -> wgpu::BindingType {
    wgpu::BindingType::Texture {
        sample_type: wgpu::TextureSampleType::Float { filterable },
        view_dimension: wgpu::TextureViewDimension::D2,
        multisampled: false,
    }
}

impl ExposureState {
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue, sampler: &wgpu::Sampler) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("exposure"),
            entries: &[
                layout_entry(0, float_texture(true)),
                layout_entry(
                    1,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                ),
                layout_entry(2, float_texture(false)),
                layout_entry(
                    3,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        });
        let shader = shaders::module(device, "exposure", shaders::EXPOSURE_SHADER);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("exposure"),
            bind_group_layouts: &[Some(&layout)],
            ..Default::default()
        });
        let make = |entry: &str, format: wgpu::TextureFormat| {
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
                    targets: &[Some(format.into())],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let sample = make("fs_sample", LUMA_FORMAT);
        let reduce = make("fs_reduce", LUMA_FORMAT);
        let adapt = make("fs_adapt", EXPOSURE_FORMAT);

        let attachment =
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let grid = texture(device, "exposure grid", LUMA_FORMAT, attachment, GRID_SIZE);
        let blocks = texture(
            device,
            "exposure blocks",
            LUMA_FORMAT,
            attachment,
            BLOCK_GRID_SIZE,
        );
        let current = texture(
            device,
            "exposure",
            EXPOSURE_FORMAT,
            attachment | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
            1,
        );
        let previous = texture(
            device,
            "exposure previous",
            EXPOSURE_FORMAT,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            1,
        );
        let view_of = |texture: &wgpu::Texture| texture.create_view(&Default::default());
        let grid_view = view_of(&grid);
        let block_view = view_of(&blocks);
        let previous_view = view_of(&previous);
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("exposure params"),
            size: PARAMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view = view_of(&current);
        let state = Self {
            reduce_group: Self::group_with(
                device,
                &layout,
                sampler,
                &grid_view,
                &previous_view,
                &params,
            ),
            adapt_group: Self::group_with(
                device,
                &layout,
                sampler,
                &block_view,
                &previous_view,
                &params,
            ),
            layout,
            sampler: sampler.clone(),
            sample,
            reduce,
            adapt,
            grid_view,
            block_view,
            current,
            previous,
            previous_view,
            params,
            view,
        };
        state.set_fixed(queue);
        state
    }

    fn group_with(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        source: &wgpu::TextureView,
        previous: &wgpu::TextureView,
        params: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("exposure"),
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(previous),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params.as_entire_binding(),
                },
            ],
        })
    }

    pub(crate) fn source_group(
        &self,
        device: &wgpu::Device,
        source: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        Self::group_with(
            device,
            &self.layout,
            &self.sampler,
            source,
            &self.previous_view,
            &self.params,
        )
    }

    pub(crate) fn set_fixed(&self, queue: &wgpu::Queue) {
        let bytes = FIXED_EXPOSURE.to_le_bytes();
        for texture in [&self.current, &self.previous] {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4),
                    rows_per_image: Some(1),
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    pub(crate) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        source_group: &wgpu::BindGroup,
        dt: f32,
        reset: bool,
    ) {
        let params: Vec<u8> = [dt, f32::from(u8::from(reset)), 0.0, 0.0]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect();
        queue.write_buffer(&self.params, 0, &params);
        let mut draw =
            |view: &wgpu::TextureView, pipeline: &wgpu::RenderPipeline, group: &wgpu::BindGroup| {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("exposure"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, group, &[]);
                pass.draw(0..3, 0..1);
            };
        draw(&self.grid_view, &self.sample, source_group);
        draw(&self.block_view, &self.reduce, &self.reduce_group);
        draw(&self.view, &self.adapt, &self.adapt_group);
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.current,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.previous,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
    }
}
