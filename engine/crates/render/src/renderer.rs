use std::collections::HashMap;

use glam::{IVec3, Mat4, Vec2, Vec3};
use mesh::mesher::SectionMeshes;
use world::block::{Block, FluidKind};
use world::clouds::{CloudMask, cloud_threshold, coverage_for_rain};
use world::pos::{ColumnPos, SectionPos};
use world::section::SECTION_SIZE;

use crate::clouds::{
    CLOUD_INSTANCES, CLOUD_VERTICES, cloud_color, create_cloud_texture, upload_cloud_mask,
};
use crate::effects::Effects;
use crate::frustum::Frustum;
use crate::globals::{Globals, view_dir_matrix};
use crate::gpu_mesh::{ColumnMeshes, GpuMesh, SceneBuffers, SectionMesh, upload_section};
use crate::gpu_timer::GpuTimer;
use crate::outline::{OUTLINE_VERTEX_SIZE, OUTLINE_VERTICES, outline_bytes};
use crate::pipelines::{HDR_FORMAT, SceneLayouts, ScenePipelines, create_depth_view};
use crate::post::{HdrTargets, create_post_layout, create_post_pipeline};
use crate::rain::{
    RAIN_INSTANCES, RAIN_VERTICES, create_rain_cover_texture, create_rain_tile_texture,
    upload_rain_cover,
};
use crate::shaders;
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
    post_layout: wgpu::BindGroupLayout,
    post_pipeline: Option<wgpu::RenderPipeline>,
    hdr_targets: Option<HdrTargets>,
    cloud_texture: wgpu::Texture,
    rain_cover_texture: wgpu::Texture,
    outline_buffer: wgpu::Buffer,
    outline_block: Option<(IVec3, Block, u8)>,
    globals_buffer: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    column_meshes: HashMap<ColumnPos, ColumnMeshes>,
    buffers: SceneBuffers,
    failed_uploads: u32,
    gpu_timer: Option<GpuTimer>,
    gpu_timing: bool,
    depth_view: wgpu::TextureView,
    drawn_sections: u32,
    frame_counts: FrameCounts,
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

        let depth_view = create_depth_view(&device, width, height);

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
                    visibility: wgpu::ShaderStages::VERTEX,
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
            ],
        });
        let rain_tile_view = create_rain_tile_texture(&device, &queue);
        let rain_cover_texture = create_rain_cover_texture(&device);
        let rain_cover_view =
            rain_cover_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let cloud_texture = create_cloud_texture(&device);
        let cloud_view = cloud_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &globals_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&block_textures),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&block_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: animation_table.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&cloud_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&rain_cover_view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&rain_tile_view),
                },
            ],
        });

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
        let scene_layouts = SceneLayouts {
            plain: pipeline_layout,
            blocks: block_layout,
        };
        let outline_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("outline"),
            size: (OUTLINE_VERTICES * OUTLINE_VERTEX_SIZE) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sdr_pipelines = ScenePipelines::new(&device, &scene_layouts, &shader, render_format);
        let post_layout = create_post_layout(&device);

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
            post_layout,
            post_pipeline: None,
            hdr_targets: None,
            cloud_texture,
            rain_cover_texture,
            outline_buffer,
            outline_block: None,
            globals_buffer,
            globals_bind_group,
            column_meshes: HashMap::new(),
            buffers,
            failed_uploads: 0,
            gpu_timer,
            gpu_timing: false,
            depth_view,
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
        self.depth_view = create_depth_view(&self.device, width, height);
        if self.hdr_targets.is_some() {
            self.hdr_targets = Some(HdrTargets::new(
                &self.device,
                &self.post_layout,
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
                &self.post_layout,
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
                &self.post_layout,
                self.render_format,
            ));
        }
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
            effects: params.effects.bits(),
            cloud_color: cloud_color(params.night, params.warm, params.rain, params.flash),
            rain: params.rain,
            rain_drops: params.rain_drops,
            snow: params.snow,
            flash: params.flash,
            cloud_threshold: f32::from(cloud_threshold(coverage_for_rain(params.rain))),
            resolution: Vec2::new(self.config.width as f32, self.config.height as f32),
            near: params.near,
            far: params.far,
            sun_screen: Vec2::ZERO,
            sun_ray: 0.0,
        };
        self.queue
            .write_buffer(&self.globals_buffer, 0, &globals.to_bytes());
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
        let hdr = self.hdr_targets.as_ref();
        let pipes = match (&self.hdr_pipelines, hdr) {
            (Some(pipes), Some(_)) => pipes,
            _ => &self.sdr_pipelines,
        };
        {
            let timer = self.gpu_timer.as_ref().filter(|_| timer_slot.is_some());
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: hdr.map_or(&view, |targets| &targets.color),
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: timer.map(|timer| {
                    if hdr.is_some() {
                        timer.begin_writes()
                    } else {
                        timer.pass_writes()
                    }
                }),
                ..Default::default()
            });
            pass.set_pipeline(&pipes.solid);
            pass.set_bind_group(0, &self.globals_bind_group, &[]);
            pass.set_bind_group(1, self.buffers.arena.bind_group(), &[]);
            pass.set_vertex_buffer(0, self.buffers.slots.buffer().slice(..));
            pass.set_index_buffer(
                self.buffers.indices.buffer().slice(..),
                wgpu::IndexFormat::Uint32,
            );
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

            let ice: Vec<&GpuMesh> = visible
                .iter()
                .rev()
                .filter_map(|(_, mesh)| mesh.ice.as_ref())
                .collect();
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

            let water: Vec<&GpuMesh> = visible
                .iter()
                .rev()
                .filter_map(|(_, mesh)| mesh.water.as_ref())
                .collect();
            if !water.is_empty() {
                pass.set_pipeline(&pipes.water);
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

    pub fn submit(&mut self, pending: PendingFrame) {
        self.queue.submit([pending.encoder.finish()]);
        if let (Some(timer), Some(slot)) = (&self.gpu_timer, pending.timer_slot) {
            timer.read_back(slot);
        }
        self.queue.present(pending.frame);
    }
}
