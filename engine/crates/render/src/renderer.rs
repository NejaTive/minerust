use std::collections::HashMap;

use glam::{IVec3, Mat4, Vec2, Vec3};
use mesh::mesher::SectionMeshes;
use world::block::{Block, FluidKind};
use world::clouds::{CloudMask, cloud_threshold, coverage_for_rain};
use world::column::WORLD_HEIGHT;
use world::pos::{ColumnPos, SectionPos};
use world::section::SECTION_SIZE;

use crate::bloom::BloomPipelines;
use crate::clouds::{
    CLOUD_INSTANCES, CLOUD_VERTICES, cloud_color, cloud_shadow_strength, create_cloud_texture,
    upload_cloud_mask,
};
use crate::effects::Effects;
use crate::frustum::Frustum;
use crate::globals::{Globals, view_dir_matrix};
use crate::gpu_mesh::{ColumnMeshes, GpuMesh, SceneBuffers, SectionMesh, upload_section};
use crate::gpu_timer::GpuTimer;
use crate::outline::{OUTLINE_VERTEX_SIZE, OUTLINE_VERTICES, outline_bytes};
use crate::pipelines::{
    HDR_FORMAT, SceneLayouts, ScenePipelines, ShadowPipelines, create_depth_target,
};
use crate::post::{
    HdrTargets, PostLayouts, create_post_pipeline, create_rays_pipeline, create_scene_copy_stub,
    light_rays,
};
use crate::rain::{
    RAIN_INSTANCES, RAIN_VERTICES, create_rain_cover_texture, create_rain_tile_texture,
    upload_rain_cover,
};
use crate::shaders;
use crate::shadow::MAX_CASCADES;
use crate::shadow::{
    ShadowData, ShadowLight, ShadowQuality, compute_cascades, quantize_light_dir, shadow_light,
};
use crate::shadow_map::{ShadowMap, comparison_sampler};
use crate::texture::{create_animation_table, create_block_textures};

pub struct FrameParams {
    pub view_proj: Mat4,
    pub camera_pos: Vec3,
    pub fog_start: f32,
    pub fog_end: f32,
    pub background: Vec3,
    pub eye_fluid: Option<FluidKind>,
    pub darkness: f32,
    pub anim_time: f32,
    pub cloud_drift: f32,
    pub sun_dir: Vec3,
    pub night: f32,
    pub zenith: Vec3,
    pub warm: f32,
    pub horizon_tilt: f32,
    pub star_rotation: Vec2,
    pub clouds: bool,
    pub effects: Effects,
    pub rain: f32,
    pub rain_drops: f32,
    pub snow: f32,
    pub flash: f32,
    pub near: f32,
    pub far: f32,
}

#[derive(Clone, Copy, Default)]
pub struct FrameCounts {
    pub solid_quads: u32,
    pub cutout_quads: u32,
    pub water_quads: u32,
    pub draw_calls: u32,
}

struct GlobalSources {
    block_textures: wgpu::TextureView,
    block_sampler: wgpu::Sampler,
    animation_table: wgpu::Buffer,
    cloud_view: wgpu::TextureView,
    rain_cover_view: wgpu::TextureView,
    rain_tile_view: wgpu::TextureView,
    shadow_buffer: wgpu::Buffer,
    shadow_sampler: wgpu::Sampler,
}

impl GlobalSources {
    fn bind_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        globals_buffer: &wgpu::Buffer,
        shadow_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.block_textures),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.block_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.animation_table.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&self.cloud_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&self.rain_cover_view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&self.rain_tile_view),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: self.shadow_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::TextureView(shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::Sampler(&self.shadow_sampler),
                },
            ],
        })
    }
}

fn uniform_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

pub struct PendingFrame {
    frame: wgpu::SurfaceTexture,
    encoder: wgpu::CommandEncoder,
    timer_slot: Option<usize>,
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_format: wgpu::TextureFormat,
    globals_layout: wgpu::BindGroupLayout,
    scene_layouts: SceneLayouts,
    shader: wgpu::ShaderModule,
    sdr_pipelines: ScenePipelines,
    hdr_pipelines: Option<ScenePipelines>,
    post_layouts: PostLayouts,
    post_pipeline: Option<wgpu::RenderPipeline>,
    rays_pipeline: Option<wgpu::RenderPipeline>,
    bloom_pipelines: Option<BloomPipelines>,
    hdr_targets: Option<HdrTargets>,
    cloud_texture: wgpu::Texture,
    rain_cover_texture: wgpu::Texture,
    outline_buffer: wgpu::Buffer,
    outline_block: Option<(IVec3, Block, u8)>,
    globals_buffer: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    global_sources: GlobalSources,
    shadow_map: ShadowMap,
    shadow_dir: Option<Vec3>,
    shadow_pipelines: ShadowPipelines,
    shadow_pass_buffers: [wgpu::Buffer; MAX_CASCADES],
    shadow_pass_groups: [wgpu::BindGroup; MAX_CASCADES],
    column_meshes: HashMap<ColumnPos, ColumnMeshes>,
    buffers: SceneBuffers,
    failed_uploads: u32,
    gpu_timer: Option<GpuTimer>,
    gpu_timing: bool,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    scene_copy_stub: wgpu::BindGroup,
    drawn_sections: u32,
    frame_counts: FrameCounts,
}

fn copy_info(texture: &wgpu::Texture) -> wgpu::TexelCopyTextureInfo<'_> {
    wgpu::TexelCopyTextureInfo {
        texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
    }
}

fn scene_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    label: &str,
    color: &wgpu::TextureView,
    depth: &wgpu::TextureView,
    clear: Option<wgpu::Color>,
    timestamp_writes: Option<wgpu::RenderPassTimestampWrites<'_>>,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: color,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: clear.map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth,
            depth_ops: Some(wgpu::Operations {
                load: clear.map_or(wgpu::LoadOp::Load, |_| wgpu::LoadOp::Clear(1.0)),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes,
        ..Default::default()
    })
}

impl Renderer {
    pub async fn new(
        surface: wgpu::Surface<'static>,
        adapter: &wgpu::Adapter,
        width: u32,
        height: u32,
    ) -> Result<Renderer, String> {
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("device"),
                required_features: adapter.features() & wgpu::Features::TIMESTAMP_QUERY,
                required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("WebGPU device is unavailable: {e}"))?;

        let caps = surface.get_capabilities(adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let render_format = format.add_srgb_suffix();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            color_space: wgpu::SurfaceColorSpace::Auto,
            view_formats: if render_format == format {
                vec![]
            } else {
                vec![render_format]
            },
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let (depth_texture, depth_view) = create_depth_target(&device, width, height);

        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: Globals::SIZE as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let block_textures = create_block_textures(&device, &queue);
        let animation_table = create_animation_table(&device);
        let block_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("blocks"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
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
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                uniform_entry(7, wgpu::ShaderStages::FRAGMENT),
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 9,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });
        let rain_tile_view = create_rain_tile_texture(&device, &queue);
        let rain_cover_texture = create_rain_cover_texture(&device);
        let rain_cover_view =
            rain_cover_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let cloud_texture = create_cloud_texture(&device);
        let cloud_view = cloud_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let shadow_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadows"),
            size: ShadowData::SIZE as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let global_sources = GlobalSources {
            block_textures,
            block_sampler,
            animation_table,
            cloud_view,
            rain_cover_view,
            rain_tile_view,
            shadow_buffer,
            shadow_sampler: comparison_sampler(&device),
        };
        let shadow_map = ShadowMap::new(&device, None);
        let globals_bind_group =
            global_sources.bind_group(&device, &globals_layout, &globals_buffer, &shadow_map.view);

        let shader = shaders::module(&device, "shader", shaders::BLOCK_SHADER);
        let buffers = SceneBuffers::new(&device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("main"),
            bind_group_layouts: &[Some(&globals_layout)],
            ..Default::default()
        });
        let block_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("blocks"),
            bind_group_layouts: &[
                Some(&globals_layout),
                Some(buffers.arena.bind_group_layout()),
            ],
            ..Default::default()
        });
        let post_layouts = PostLayouts::new(&device);
        let scene_copy_stub = create_scene_copy_stub(&device, &post_layouts);
        let water_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("water"),
            bind_group_layouts: &[
                Some(&globals_layout),
                Some(buffers.arena.bind_group_layout()),
                Some(&post_layouts.scene_copy),
            ],
            ..Default::default()
        });
        let scene_layouts = SceneLayouts {
            plain: pipeline_layout,
            blocks: block_layout,
            water: water_layout,
        };
        let outline_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("outline"),
            size: (OUTLINE_VERTICES * OUTLINE_VERTEX_SIZE) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shadow_pass_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("shadow pass"),
                entries: &[
                    uniform_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT),
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2Array,
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
                    uniform_entry(3, wgpu::ShaderStages::VERTEX_FRAGMENT),
                    uniform_entry(10, wgpu::ShaderStages::VERTEX),
                ],
            });
        let shadow_pass_layout_pipeline =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("shadow"),
                bind_group_layouts: &[
                    Some(&shadow_pass_layout),
                    Some(buffers.arena.bind_group_layout()),
                ],
                ..Default::default()
            });
        let shadow_pipelines = ShadowPipelines::new(&device, &shadow_pass_layout_pipeline, &shader);
        let shadow_pass_buffers: [wgpu::Buffer; MAX_CASCADES] = std::array::from_fn(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("shadow pass"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        let shadow_pass_groups = std::array::from_fn(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shadow pass"),
                layout: &shadow_pass_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: globals_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            &global_sources.block_textures,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&global_sources.block_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: global_sources.animation_table.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 10,
                        resource: shadow_pass_buffers[i].as_entire_binding(),
                    },
                ],
            })
        });
        let sdr_pipelines = ScenePipelines::new(&device, &scene_layouts, &shader, render_format);

        let gpu_timer = device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY)
            .then(|| GpuTimer::new(&device, &queue));

        Ok(Renderer {
            surface,
            device,
            queue,
            config,
            render_format,
            globals_layout,
            scene_layouts,
            shader,
            sdr_pipelines,
            hdr_pipelines: None,
            post_layouts,
            post_pipeline: None,
            rays_pipeline: None,
            bloom_pipelines: None,
            hdr_targets: None,
            cloud_texture,
            rain_cover_texture,
            outline_buffer,
            outline_block: None,
            globals_buffer,
            globals_bind_group,
            global_sources,
            shadow_map,
            shadow_dir: None,
            shadow_pipelines,
            shadow_pass_buffers,
            shadow_pass_groups,
            column_meshes: HashMap::new(),
            buffers,
            failed_uploads: 0,
            gpu_timer,
            gpu_timing: false,
            depth_texture,
            depth_view,
            scene_copy_stub,
            drawn_sections: 0,
            frame_counts: FrameCounts::default(),
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        (self.depth_texture, self.depth_view) = create_depth_target(&self.device, width, height);
        if self.hdr_targets.is_some() {
            self.hdr_targets = Some(HdrTargets::new(
                &self.device,
                &self.post_layouts,
                width,
                height,
            ));
        }
    }

    fn sync_hdr(&mut self, enabled: bool) {
        if !enabled {
            self.hdr_targets = None;
            return;
        }
        if self.hdr_targets.is_none() {
            self.hdr_targets = Some(HdrTargets::new(
                &self.device,
                &self.post_layouts,
                self.config.width,
                self.config.height,
            ));
        }
        if self.hdr_pipelines.is_none() {
            self.hdr_pipelines = Some(ScenePipelines::new(
                &self.device,
                &self.scene_layouts,
                &self.shader,
                HDR_FORMAT,
            ));
            self.post_pipeline = Some(create_post_pipeline(
                &self.device,
                &self.globals_layout,
                &self.post_layouts.post,
                self.render_format,
            ));
            self.rays_pipeline = Some(create_rays_pipeline(
                &self.device,
                &self.globals_layout,
                &self.post_layouts.scene_copy,
            ));
            self.bloom_pipelines =
                Some(BloomPipelines::new(&self.device, &self.post_layouts.bloom));
        }
    }

    fn sync_shadow_map(&mut self, quality: Option<ShadowQuality>) {
        if self.shadow_map.quality == quality {
            return;
        }
        self.shadow_map = ShadowMap::new(&self.device, quality);
        self.globals_bind_group = self.global_sources.bind_group(
            &self.device,
            &self.globals_layout,
            &self.globals_buffer,
            &self.shadow_map.view,
        );
    }

    pub fn set_cloud_mask(&self, mask: &CloudMask) {
        upload_cloud_mask(&self.queue, &self.cloud_texture, mask);
    }

    pub fn set_rain_cover(&self, heights: &[u8]) {
        upload_rain_cover(&self.queue, &self.rain_cover_texture, heights);
    }

    pub fn aspect(&self) -> f32 {
        self.config.width as f32 / self.config.height as f32
    }

    pub fn upload_column(&mut self, key: ColumnPos, data: &[SectionMeshes]) {
        self.remove_column(key);
        let origin = key.origin();
        let meshes = std::array::from_fn(|sy| {
            let min = (origin + IVec3::Y * sy as i32 * SECTION_SIZE).as_vec3();
            self.upload(&data[sy], min)
        });
        self.column_meshes.insert(key, meshes);
    }

    pub fn upload_section(&mut self, key: SectionPos, data: &SectionMeshes) {
        let column = key.column();
        if !self.column_meshes.contains_key(&column) {
            return;
        }
        let min = key.origin().as_vec3();
        let new = self.upload(data, min);
        let old = self
            .column_meshes
            .get_mut(&column)
            .map(|meshes| std::mem::replace(&mut meshes[key.y() as usize], new));
        if let Some(old) = old.flatten() {
            old.release(&mut self.buffers);
        }
    }

    fn upload(&mut self, data: &SectionMeshes, min: Vec3) -> Option<SectionMesh> {
        match upload_section(&self.device, &self.queue, &mut self.buffers, data, min) {
            Ok(mesh) => mesh,
            Err(_) => {
                self.failed_uploads += 1;
                None
            }
        }
    }

    pub fn remove_column(&mut self, key: ColumnPos) {
        if let Some(meshes) = self.column_meshes.remove(&key) {
            for mesh in meshes.into_iter().flatten() {
                mesh.release(&mut self.buffers);
            }
        }
    }

    pub fn failed_uploads(&self) -> u32 {
        self.failed_uploads
    }

    pub fn has_column(&self, key: ColumnPos) -> bool {
        self.column_meshes.contains_key(&key)
    }

    pub fn column_keys(&self) -> impl Iterator<Item = ColumnPos> + '_ {
        self.column_meshes.keys().copied()
    }

    pub fn column_count(&self) -> usize {
        self.column_meshes.len()
    }

    pub fn section_count(&self) -> usize {
        self.column_meshes.values().flatten().flatten().count()
    }

    pub fn gpu_bytes(&self) -> u64 {
        self.buffers.bytes()
    }

    pub fn gpu_used_bytes(&self) -> u64 {
        self.buffers.used_bytes()
    }

    pub fn has_gpu_timer(&self) -> bool {
        self.gpu_timer.is_some()
    }

    pub fn set_gpu_timing(&mut self, enabled: bool) {
        self.gpu_timing = enabled;
    }

    pub fn take_gpu_samples(&self) -> Option<(f64, u32)> {
        self.gpu_timer.as_ref()?.take_samples()
    }

    pub fn drawn_sections(&self) -> u32 {
        self.drawn_sections
    }

    pub fn frame_counts(&self) -> FrameCounts {
        self.frame_counts
    }

    pub fn set_outline(&mut self, outline: Option<(IVec3, Block, u8)>) {
        if outline != self.outline_block {
            if let Some((pos, block, data)) = outline {
                self.queue.write_buffer(
                    &self.outline_buffer,
                    0,
                    &outline_bytes(pos, block.bounds(data)),
                );
            }
            self.outline_block = outline;
        }
    }

    pub fn begin(&mut self, params: &FrameParams) -> Result<Option<PendingFrame>, String> {
        let (sun_screen, sun_ray) = if params.effects.hdr && params.effects.sun_rays {
            light_rays(
                params.view_proj,
                params.camera_pos,
                params.sun_dir,
                params.night,
                params.rain,
                params.eye_fluid.is_some(),
            )
        } else {
            (Vec2::ZERO, 0.0)
        };
        let sky_light = shadow_light(params.sun_dir, params.warm, params.rain);
        let light = sky_light
            .filter(|_| params.effects.shadows)
            .map(|mut light| {
                light.dir = quantize_light_dir(self.shadow_dir, light.dir);
                light
            });
        self.shadow_dir = light.map(|light| light.dir);
        let quality = params.effects.shadow_quality;
        self.sync_shadow_map(light.map(|_| quality));
        let cloud_strength = sky_light
            .filter(|_| params.effects.cloud_shadows)
            .map_or(0.0, |sky| cloud_shadow_strength(sky.strength, params.rain));
        let cloud_light = sky_light.map(|sky| ShadowLight {
            dir: light.map_or(sky.dir, |block| block.dir),
            strength: sky.strength,
        });
        let shadow = light.map(|light| ShadowData {
            cascades: compute_cascades(
                params.view_proj,
                params.near,
                params.fog_end.min(params.far),
                light.dir,
                quality,
                WORLD_HEIGHT as f32,
            ),
            light,
            quality,
            cloud_strength,
        });
        let effects = Effects {
            shadows: shadow.is_some(),
            cloud_shadows: cloud_strength > 0.0,
            ..params.effects
        };
        let [r, g, b] = params.background.to_array().map(f64::from);
        let clear = wgpu::Color { r, g, b, a: 1.0 };
        let globals = Globals {
            view_proj: params.view_proj,
            camera_pos: params.camera_pos,
            fog_start: params.fog_start,
            fog_color: params.background,
            fog_end: params.fog_end,
            eye_fluid: match params.eye_fluid {
                None => 0.0,
                Some(FluidKind::Water) => 1.0,
                Some(FluidKind::Lava) => 2.0,
            },
            darkness: params.darkness,
            anim_time: params.anim_time,
            cloud_drift: params.cloud_drift,
            view_dir: view_dir_matrix(params.view_proj),
            sun_dir: params.sun_dir,
            night: params.night,
            zenith: params.zenith,
            warm: params.warm,
            horizon_tilt: params.horizon_tilt,
            star_rotation: params.star_rotation,
            effects: effects.bits(),
            cloud_color: cloud_color(params.night, params.warm, params.rain, params.flash),
            rain: params.rain,
            rain_drops: params.rain_drops,
            snow: params.snow,
            flash: params.flash,
            cloud_threshold: f32::from(cloud_threshold(coverage_for_rain(params.rain))),
            resolution: Vec2::new(self.config.width as f32, self.config.height as f32),
            near: params.near,
            far: params.far,
            sun_screen,
            sun_ray,
        };
        self.queue
            .write_buffer(&self.globals_buffer, 0, &globals.to_bytes());
        if let Some(shadow) = &shadow {
            self.queue
                .write_buffer(&self.global_sources.shadow_buffer, 0, &shadow.to_bytes());
            for (buffer, matrix) in self
                .shadow_pass_buffers
                .iter()
                .zip(shadow.cascades.matrices)
            {
                let bytes: Vec<u8> = matrix
                    .to_cols_array()
                    .into_iter()
                    .flat_map(f32::to_le_bytes)
                    .collect();
                self.queue.write_buffer(buffer, 0, &bytes);
            }
        } else if let (Some(light), true) = (cloud_light, cloud_strength > 0.0) {
            let data = ShadowData::cloud_only(light, cloud_strength);
            self.queue
                .write_buffer(&self.global_sources.shadow_buffer, 0, &data.to_bytes());
        }
        let frustum = Frustum::from_view_proj(params.view_proj);

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.surface.configure(&self.device, &self.config);
                frame
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(None);
            }
            other => return Err(format!("surface error: {other:?}")),
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.render_format),
            ..Default::default()
        });
        self.sync_hdr(params.effects.hdr);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        let timer_slot = match &mut self.gpu_timer {
            Some(timer) if self.gpu_timing => timer.acquire(),
            _ => None,
        };
        if let Some(shadow) = &shadow {
            self.encode_shadows(&mut encoder, shadow);
        }
        let hdr = self.hdr_targets.as_ref();
        let split;
        let pipes = match (&self.hdr_pipelines, hdr) {
            (Some(pipes), Some(_)) => pipes,
            _ => &self.sdr_pipelines,
        };
        {
            let timer = self.gpu_timer.as_ref().filter(|_| timer_slot.is_some());
            let eye = params.camera_pos;
            let mut visible: Vec<(f32, &SectionMesh)> = self
                .column_meshes
                .values()
                .flatten()
                .flatten()
                .filter(|mesh| frustum.intersects_aabb(mesh.min, mesh.max))
                .map(|mesh| (((mesh.min + mesh.max) / 2.0).distance_squared(eye), mesh))
                .collect();
            visible.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
            let ice: Vec<&GpuMesh> = visible
                .iter()
                .rev()
                .filter_map(|(_, mesh)| mesh.ice.as_ref())
                .collect();
            let water: Vec<&GpuMesh> = visible
                .iter()
                .rev()
                .filter_map(|(_, mesh)| mesh.water.as_ref())
                .collect();
            let scene_copy_needed = params.effects.water_depth
                || (params.effects.ssr && params.effects.water_reflections);
            split = hdr.is_some() && scene_copy_needed && !water.is_empty();
            let scene_color = hdr.map_or(&view, |targets| &targets.color);
            let mut pass = scene_pass(
                &mut encoder,
                "main",
                scene_color,
                &self.depth_view,
                Some(clear),
                timer.map(|timer| {
                    if hdr.is_some() {
                        timer.begin_writes()
                    } else {
                        timer.pass_writes()
                    }
                }),
            );
            pass.set_pipeline(&pipes.solid);
            pass.set_bind_group(0, &self.globals_bind_group, &[]);
            pass.set_bind_group(1, self.buffers.arena.bind_group(), &[]);
            pass.set_vertex_buffer(0, self.buffers.slots.buffer().slice(..));
            pass.set_index_buffer(
                self.buffers.indices.buffer().slice(..),
                wgpu::IndexFormat::Uint32,
            );
            let mut counts = FrameCounts::default();
            for (_, section) in &visible {
                if let Some(mesh) = &section.solid {
                    mesh.draw(&mut pass);
                    counts.solid_quads += mesh.quad_count;
                    counts.draw_calls += 1;
                }
            }
            pass.set_pipeline(&pipes.cutout);
            for (_, section) in &visible {
                if let Some(mesh) = &section.cutout {
                    mesh.draw(&mut pass);
                    counts.cutout_quads += mesh.quad_count;
                    counts.draw_calls += 1;
                }
            }
            self.drawn_sections = visible.len() as u32;

            if self.outline_block.is_some() {
                pass.set_pipeline(&pipes.outline);
                pass.set_vertex_buffer(0, self.outline_buffer.slice(..));
                pass.draw(0..OUTLINE_VERTICES as u32, 0..1);
                counts.draw_calls += 1;
            }

            if params.eye_fluid.is_none() {
                pass.set_pipeline(&pipes.sky);
                pass.draw(0..3, 0..1);
                counts.draw_calls += 1;
            }

            if params.clouds && params.eye_fluid.is_none() {
                pass.set_pipeline(&pipes.cloud);
                pass.draw(0..CLOUD_VERTICES, 0..CLOUD_INSTANCES);
                counts.draw_calls += 1;
            }

            if params.rain_drops > 0.0 && params.eye_fluid.is_none() {
                pass.set_pipeline(&pipes.rain);
                pass.draw(0..RAIN_VERTICES, 0..RAIN_INSTANCES);
                counts.draw_calls += 1;
            }

            if split && let Some(targets) = hdr {
                drop(pass);
                let size = wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                };
                encoder.copy_texture_to_texture(
                    copy_info(&targets.color_texture),
                    copy_info(&targets.color_copy),
                    size,
                );
                encoder.copy_texture_to_texture(
                    copy_info(&self.depth_texture),
                    copy_info(&targets.depth_copy),
                    size,
                );
                pass = scene_pass(
                    &mut encoder,
                    "water",
                    scene_color,
                    &self.depth_view,
                    None,
                    None,
                );
                pass.set_bind_group(0, &self.globals_bind_group, &[]);
                pass.set_index_buffer(
                    self.buffers.indices.buffer().slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.set_bind_group(2, &targets.scene_copy_group, &[]);
            }
            if !ice.is_empty() {
                pass.set_bind_group(1, self.buffers.arena.bind_group(), &[]);
                pass.set_vertex_buffer(0, self.buffers.slots.buffer().slice(..));
                for pipeline in [&pipes.ice_depth, &pipes.ice] {
                    pass.set_pipeline(pipeline);
                    for mesh in &ice {
                        mesh.draw(&mut pass);
                        counts.draw_calls += 1;
                    }
                }
                counts.water_quads += ice.iter().map(|mesh| mesh.quad_count).sum::<u32>();
            }

            if !water.is_empty() {
                pass.set_pipeline(&pipes.water);
                if !split {
                    pass.set_bind_group(2, &self.scene_copy_stub, &[]);
                }
                pass.set_bind_group(1, self.buffers.arena.bind_group(), &[]);
                pass.set_vertex_buffer(0, self.buffers.slots.buffer().slice(..));
                for mesh in water {
                    mesh.draw(&mut pass);
                    counts.water_quads += mesh.quad_count;
                    counts.draw_calls += 1;
                }
            }
            self.frame_counts = counts;
        }
        if let (Some(targets), Some(rays)) = (hdr, &self.rays_pipeline)
            && sun_ray > 0.0
        {
            if !split {
                encoder.copy_texture_to_texture(
                    copy_info(&self.depth_texture),
                    copy_info(&targets.depth_copy),
                    wgpu::Extent3d {
                        width: self.config.width,
                        height: self.config.height,
                        depth_or_array_layers: 1,
                    },
                );
            }
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sun rays"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.rays,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(rays);
            pass.set_bind_group(0, &self.globals_bind_group, &[]);
            pass.set_bind_group(1, &targets.scene_copy_group, &[]);
            pass.draw(0..3, 0..1);
        }
        if let (Some(targets), Some(bloom)) = (hdr, &self.bloom_pipelines)
            && params.effects.bloom
        {
            targets.bloom.encode(&mut encoder, bloom);
        }
        if let Some(targets) = hdr {
            let timer = self.gpu_timer.as_ref().filter(|_| timer_slot.is_some());
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("post"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: timer.map(GpuTimer::end_writes),
                ..Default::default()
            });
            if let Some(post) = &self.post_pipeline {
                pass.set_pipeline(post);
                pass.set_bind_group(0, &self.globals_bind_group, &[]);
                pass.set_bind_group(1, &targets.post_group, &[]);
                pass.draw(0..3, 0..1);
            }
        }
        if let (Some(timer), Some(slot)) = (&self.gpu_timer, timer_slot) {
            timer.resolve(&mut encoder, slot);
        }
        Ok(Some(PendingFrame {
            frame,
            encoder,
            timer_slot,
        }))
    }

    fn encode_shadows(&self, encoder: &mut wgpu::CommandEncoder, shadow: &ShadowData) {
        for cascade in 0..shadow.cascades.count {
            let frustum = Frustum::from_view_proj(shadow.cascades.matrices[cascade]);
            let casters: Vec<&SectionMesh> = self
                .column_meshes
                .values()
                .flatten()
                .flatten()
                .filter(|mesh| frustum.intersects_aabb(mesh.min, mesh.max))
                .collect();
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_map.layers[cascade],
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, &self.shadow_pass_groups[cascade], &[]);
            pass.set_bind_group(1, self.buffers.arena.bind_group(), &[]);
            pass.set_vertex_buffer(0, self.buffers.slots.buffer().slice(..));
            pass.set_index_buffer(
                self.buffers.indices.buffer().slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.set_pipeline(&self.shadow_pipelines.solid);
            for mesh in casters.iter().filter_map(|section| section.solid.as_ref()) {
                mesh.draw(&mut pass);
            }
            pass.set_pipeline(&self.shadow_pipelines.cutout);
            for mesh in casters.iter().filter_map(|section| section.cutout.as_ref()) {
                mesh.draw(&mut pass);
            }
        }
    }

    pub fn submit(&mut self, pending: PendingFrame) {
        self.queue.submit([pending.encoder.finish()]);
        if let (Some(timer), Some(slot)) = (&self.gpu_timer, pending.timer_slot) {
            timer.read_back(slot);
        }
        self.queue.present(pending.frame);
    }
}
