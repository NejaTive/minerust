use std::collections::HashMap;

use glam::{IVec2, IVec3, Mat4, Vec2, Vec3};
use mesh::mesh::quad_index_bytes;
use mesh::mesher::SectionMeshes;
use wgpu::util::DeviceExt;
use world::block::Block;
use world::section::SECTION_SIZE;

use crate::frustum::Frustum;
use crate::globals::{Globals, view_dir_matrix};
use crate::gpu_mesh::{ColumnMeshes, GpuMesh, SectionMesh, upload_section};
use crate::outline::{OUTLINE_VERTEX_SIZE, OUTLINE_VERTICES, outline_bytes};
use crate::pipelines::{
    BlockPass, create_block_pipeline, create_depth_view, create_outline_pipeline,
    create_sky_pipeline,
};
use crate::shaders;
use crate::texture::create_block_textures;

/// Всё, что нужно кадру от игры: `render` не знает про `game`, заполняет это `web`.
pub struct FrameParams {
    pub view_proj: Mat4,
    pub camera_pos: Vec3,
    /// Горизонтальное расстояние в блоках, с которого начинается туман.
    pub fog_start: f32,
    /// Горизонтальное расстояние в блоках, с которого виден только туман.
    pub fog_end: f32,
    /// Линейный цвет фона, тумана и неба у горизонта.
    pub background: Vec3,
    /// Глаза под водой: тон и туман по полному расстоянию, неба нет.
    pub underwater: bool,
    /// Уровней солнечного света, которые отнимает ночь.
    pub darkness: f32,
    pub sun_dir: Vec3,
    /// Степень ночи 0..1: видимость звёзд.
    pub night: f32,
    /// Линейный цвет неба в зените.
    pub zenith: Vec3,
    /// Вес тёплого пятна заката у горизонта 0..1.
    pub warm: f32,
    /// Насколько край земли ниже горизонта, см. `Globals::horizon_tilt`.
    pub horizon_tilt: f32,
    /// (cos, sin) поворота звёзд.
    pub star_rotation: Vec2,
}

/// Счётчики последнего кадра для `stats()`.
#[derive(Clone, Copy, Default)]
pub struct FrameCounts {
    pub solid_vertices: u32,
    pub solid_indices: u32,
    pub water_vertices: u32,
    pub water_indices: u32,
    pub draw_calls: u32,
}

/// Кадр, записанный в команды, но не отправленный: между [`Renderer::begin`] и
/// [`Renderer::submit`] вызывающий может снять время кодирования.
pub struct PendingFrame {
    frame: wgpu::SurfaceTexture,
    encoder: wgpu::CommandEncoder,
}

/// Устройство, surface, пайплайны и GPU-меши колонок; рисует кадры.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    /// Формат view кадра: всегда sRGB, даже если поверхность без него.
    render_format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    /// Вода: со смешением, без записи глубины, после рамки.
    water_pipeline: wgpu::RenderPipeline,
    outline_pipeline: wgpu::RenderPipeline,
    sky_pipeline: wgpu::RenderPipeline,
    /// Рёбра рамки вокруг `outline_block`.
    outline_buffer: wgpu::Buffer,
    /// Индексы квадов `0,1,2, 0,2,3` для всех мешей секций ([`quad_index_bytes`]):
    /// диагональ выбирается порядком вершин, так что своих индексов у секции нет.
    quad_index_buffer: wgpu::Buffer,
    /// Блок, чья рамка лежит в `outline_buffer` и рисуется; `None` — рамки нет.
    outline_block: Option<(IVec3, Block)>,
    globals_buffer: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    /// Меши колонок по ключу колонки, по секции снизу вверх; `None` — пустая секция.
    column_meshes: HashMap<IVec2, ColumnMeshes>,
    depth_view: wgpu::TextureView,
    /// Сколько секций прошло отсечение по пирамиде видимости в последнем кадре.
    drawn_sections: u32,
    /// Счётчики последнего кадра; считаются всегда, это сложение целых.
    frame_counts: FrameCounts,
}

impl Renderer {
    /// Устройство, очередь и пайплайны для `surface`; `adapter` должен быть создан с
    /// `compatible_surface: Some(&surface)`. Surface создаёт вызывающий: `SurfaceTarget`
    /// canvas есть только на wasm32.
    pub async fn new(
        surface: wgpu::Surface<'static>,
        adapter: &wgpu::Adapter,
        width: u32,
        height: u32,
    ) -> Result<Renderer, String> {
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("device"),
                required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("WebGPU device is unavailable: {e}"))?;

        let caps = surface.get_capabilities(adapter);
        // Канвас WebGPU не умеет sRGB-форматы поверхности (только `*8unorm`), зато
        // разрешает sRGB-view через `view_formats`. Без него линейный цвет шейдера
        // ушёл бы на экран без перевода в sRGB, и картинка потемнела бы.
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
        // Nearest внутри уровня — чёткие пиксели вблизи, как в Minecraft; Linear между
        // мип-уровнями — без ряби вдали и без видимой границы смены уровня.
        // Repeat — UV задаются в блоках, так грань больше 1×1 (greedy meshing)
        // повторит тайл без правок шейдера.
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
                    // Фрагментному шейдеру нужны параметры тумана.
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
            ],
        });
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
            ],
        });

        let shader = shaders::module(&device, "shader", shaders::BLOCK_SHADER);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("main"),
            bind_group_layouts: &[Some(&globals_layout)],
            ..Default::default()
        });
        let outline_pipeline = create_outline_pipeline(&device, &pipeline_layout, render_format);
        let sky_pipeline = create_sky_pipeline(&device, &pipeline_layout, render_format);
        let outline_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("outline"),
            size: (OUTLINE_VERTICES * OUTLINE_VERTEX_SIZE) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let quad_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad indices"),
            contents: &quad_index_bytes(),
            usage: wgpu::BufferUsages::INDEX,
        });
        let pipeline = create_block_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            render_format,
            BlockPass::Solid,
        );
        let water_pipeline = create_block_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            render_format,
            BlockPass::Water,
        );

        Ok(Renderer {
            surface,
            device,
            queue,
            config,
            render_format,
            pipeline,
            water_pipeline,
            outline_pipeline,
            sky_pipeline,
            outline_buffer,
            quad_index_buffer,
            outline_block: None,
            globals_buffer,
            globals_bind_group,
            column_meshes: HashMap::new(),
            depth_view,
            drawn_sections: 0,
            frame_counts: FrameCounts::default(),
        })
    }

    /// Вызывается при изменении размеров canvas (в физических пикселях).
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.depth_view = create_depth_view(&self.device, width, height);
    }

    pub fn aspect(&self) -> f32 {
        self.config.width as f32 / self.config.height as f32
    }

    /// Загружает меши колонки на GPU (секции снизу вверх), заменяя прежние. Пустые
    /// секции буферов не занимают.
    pub fn upload_column(&mut self, key: IVec2, data: &[SectionMeshes]) {
        let origin = IVec3::new(key.x * SECTION_SIZE, 0, key.y * SECTION_SIZE);
        let meshes = std::array::from_fn(|sy| {
            let min = (origin + IVec3::Y * sy as i32 * SECTION_SIZE).as_vec3();
            upload_section(&self.device, &data[sy], min)
        });
        self.column_meshes.insert(key, meshes);
    }

    /// Заменяет меш одной секции `(x колонки, номер секции, z колонки)`. Колонку без
    /// меша пропускает: её загрузят целиком позже.
    pub fn upload_section(&mut self, key: IVec3, data: &SectionMeshes) {
        let Some(meshes) = self.column_meshes.get_mut(&IVec2::new(key.x, key.z)) else {
            return;
        };
        let min = (key * SECTION_SIZE).as_vec3();
        meshes[key.y as usize] = upload_section(&self.device, data, min);
    }

    /// Освобождает буферы колонки.
    pub fn remove_column(&mut self, key: IVec2) {
        self.column_meshes.remove(&key);
    }

    pub fn has_column(&self, key: IVec2) -> bool {
        self.column_meshes.contains_key(&key)
    }

    pub fn column_keys(&self) -> impl Iterator<Item = IVec2> + '_ {
        self.column_meshes.keys().copied()
    }

    pub fn column_count(&self) -> usize {
        self.column_meshes.len()
    }

    /// Секций с мешем на GPU.
    pub fn section_count(&self) -> usize {
        self.column_meshes.values().flatten().flatten().count()
    }

    /// Память мешей и общих индексов на GPU, байт.
    pub fn gpu_bytes(&self) -> u64 {
        self.column_meshes
            .values()
            .flatten()
            .flatten()
            .map(SectionMesh::bytes)
            .sum::<u64>()
            + self.quad_index_buffer.size()
    }

    /// Секций, прошедших отсечение в последнем кадре.
    pub fn drawn_sections(&self) -> u32 {
        self.drawn_sections
    }

    pub fn frame_counts(&self) -> FrameCounts {
        self.frame_counts
    }

    /// Рамка вокруг блока под прицелом; `None` убирает её. Блок в ключе: на месте
    /// цветка мог появиться камень, и рамка стала бы другой.
    pub fn set_outline(&mut self, outline: Option<(IVec3, Block)>) {
        if outline != self.outline_block {
            if let Some((pos, block)) = outline {
                self.queue.write_buffer(
                    &self.outline_buffer,
                    0,
                    &outline_bytes(pos, block.bounds()),
                );
            }
            self.outline_block = outline;
        }
    }

    /// Записывает кадр в команды. `Ok(None)` — кадр пропущен (поверхность устарела или
    /// закрыта), следующий попробует снова.
    pub fn begin(&mut self, params: &FrameParams) -> Result<Option<PendingFrame>, String> {
        let [r, g, b] = params.background.to_array().map(f64::from);
        let clear = wgpu::Color { r, g, b, a: 1.0 };
        let globals = Globals {
            view_proj: params.view_proj,
            camera_pos: params.camera_pos,
            fog_start: params.fog_start,
            fog_color: params.background,
            fog_end: params.fog_end,
            underwater: f32::from(u8::from(params.underwater)),
            darkness: params.darkness,
            view_dir: view_dir_matrix(params.view_proj),
            sun_dir: params.sun_dir,
            night: params.night,
            zenith: params.zenith,
            warm: params.warm,
            horizon_tilt: params.horizon_tilt,
            star_rotation: params.star_rotation,
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
            // Пропускаем кадр и пробуем на следующем.
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(None);
            }
            other => return Err(format!("surface error: {other:?}")),
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.render_format),
            ..Default::default()
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_index_buffer(self.quad_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.set_bind_group(0, &self.globals_bind_group, &[]);
            let visible: Vec<&SectionMesh> = self
                .column_meshes
                .values()
                .flatten()
                .flatten()
                .filter(|mesh| frustum.intersects_aabb(mesh.min, mesh.max))
                .collect();
            let mut counts = FrameCounts::default();
            for (section, mesh) in visible
                .iter()
                .filter_map(|section| Some((*section, section.solid.as_ref()?)))
            {
                mesh.draw(&mut pass, &section.origin);
                counts.solid_vertices += mesh.vertex_count;
                counts.solid_indices += mesh.index_count;
                counts.draw_calls += 1;
            }
            self.drawn_sections = visible.len() as u32;

            // После рельефа: глубина `LessEqual` без записи прячет рёбра за блоками.
            if self.outline_block.is_some() {
                pass.set_pipeline(&self.outline_pipeline);
                pass.set_vertex_buffer(0, self.outline_buffer.slice(..));
                pass.draw(0..OUTLINE_VERTICES as u32, 0..1);
                counts.draw_calls += 1;
            }

            // Небо закрашивает только пустые пиксели (глубина 1.0), поэтому после рельефа и
            // рамки, но до воды: вода смешивается уже с небом. Под водой неба нет, фон —
            // подводный цвет.
            if !params.underwater {
                pass.set_pipeline(&self.sky_pipeline);
                pass.draw(0..3, 0..1);
                counts.draw_calls += 1;
            }

            // Вода — последней и от дальних секций к ближним: глубину она не пишет,
            // и смешение верно, только если ближняя вода ложится поверх дальней.
            // Внутри секции грани не сортируются: цвет у воды один, и разница почти
            // не видна.
            let eye = params.camera_pos;
            let mut water: Vec<(f32, &SectionMesh, &GpuMesh)> = visible
                .iter()
                .filter_map(|mesh| {
                    let center = (mesh.min + mesh.max) / 2.0;
                    Some((center.distance_squared(eye), *mesh, mesh.water.as_ref()?))
                })
                .collect();
            if !water.is_empty() {
                water.sort_by(|a, b| b.0.total_cmp(&a.0));
                pass.set_pipeline(&self.water_pipeline);
                for (_, section, mesh) in water {
                    mesh.draw(&mut pass, &section.origin);
                    counts.water_vertices += mesh.vertex_count;
                    counts.water_indices += mesh.index_count;
                    counts.draw_calls += 1;
                }
            }
            self.frame_counts = counts;
        }
        Ok(Some(PendingFrame { frame, encoder }))
    }

    /// Отправляет записанный кадр и показывает его.
    pub fn submit(&mut self, pending: PendingFrame) {
        self.queue.submit([pending.encoder.finish()]);
        self.queue.present(pending.frame);
    }
}
