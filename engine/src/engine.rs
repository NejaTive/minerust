use std::collections::{HashMap, HashSet};

use glam::{IVec2, IVec3, Vec3};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, Performance};
use wgpu::util::DeviceExt;

use crate::block::Block;
use crate::camera::Camera;
use crate::column::COLUMN_SECTIONS;
use crate::double_tap::DoubleTap;
use crate::fog::Fog;
use crate::frustum::Frustum;
use crate::globals::Globals;
use crate::loader::{Loader, NEIGHBORS, Progress};
use crate::mesh::{self, Vertex};
use crate::mesher::{MeshData, MeshOptions, mesh_column, mesh_world_section};
use crate::outline::{OUTLINE_VERTEX_SIZE, OUTLINE_VERTICES, outline_bytes};
use crate::physics::{Clock, EYE_HEIGHT, Input, Mode, Player};
use crate::raycast::{Hit, raycast};
use crate::repeat::Repeat;
use crate::save::{FORMAT_VERSION, decode_column, encode_column};
use crate::section::SECTION_SIZE;
use crate::terrain::Terrain;
use crate::texture::{self, LAYER_COUNT, MIP_LEVELS, TILE_SIZE};
use crate::world::{World, affected_sections, column_key};

/// Цвет неба в линейном пространстве: рендер идёт в sRGB-view, и при выводе он
/// станет прежним sRGB (0.53, 0.81, 0.92).
const SKY: wgpu::Color = wgpu::Color {
    r: 0.24,
    g: 0.62,
    b: 0.83,
    a: 1.0,
};

/// Круг колонок, который `create` грузит и мешует синхронно, чтобы первый кадр не был пустым.
const START_RADIUS: i32 = 2;
/// Бюджет потоковой загрузки на кадр, мс. Сам кадр без подгрузки занимает на CPU
/// ~1–1.5 мс, и 3 мс сверху оставляют запас даже при 120 Гц (8.3 мс на кадр).
/// В release это ~10 мешей колонки (~0.25 мс) за кадр: круг дальности 8 (~200 колонок)
/// заполняется быстрее, чем за секунду, а полёт 40 блоков/с требует меньше колонки
/// на кадр. Бюджет по времени, а не по числу колонок, сам подстраивается под dev-сборку
/// и слабые машины: кадр не проседает, просто круг заполняется дольше.
const STREAM_BUDGET_MS: f64 = 3.0;

/// Дальность действия игрока, блоков от камеры: как в креативе Minecraft.
const REACH: f32 = 6.0;
/// Период повтора ломания и установки при удержании кнопки, с.
const REPEAT_S: f32 = 0.25;
/// Блок в руке до первого выбора клавишами 1–3.
const DEFAULT_BLOCK: Block = Block::Grass;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// Верхняя граница шага кадра, чтобы после сворачивания вкладки камера не улетала.
/// Заодно ограничивает число шагов физики за кадр: не больше 6.
const MAX_DT: f32 = 0.1;
/// Окно двойного нажатия пробела (полёт ↔ ходьба), с.
const DOUBLE_TAP_S: f32 = 0.3;

/// Движок игры. Владеет GPU-ресурсами и рисует кадры на переданном canvas.
#[wasm_bindgen]
pub struct Engine {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    /// Формат view кадра: всегда sRGB, даже если поверхность без него.
    render_format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    outline_pipeline: wgpu::RenderPipeline,
    /// Рёбра рамки вокруг `outline_block`.
    outline_buffer: wgpu::Buffer,
    /// Блок, чья рамка сейчас лежит в `outline_buffer`.
    outline_block: Option<IVec3>,
    /// Блок под прицелом в последнем кадре; рамка рисуется вокруг него.
    target: Option<IVec3>,
    breaking: Repeat,
    placing: Repeat,
    /// Блок, который ставит правая кнопка.
    selected: Block,
    globals_buffer: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    /// Меши колонок по ключу колонки, по секции снизу вверх; `None` — пустая секция.
    column_meshes: HashMap<IVec2, ColumnMeshes>,
    mesh_options: MeshOptions,
    /// Колонки, чьи меши построены со старыми `mesh_options`, дальние первыми: их
    /// перестраивает `stream_columns` в бюджете кадра, с конца. До замены рисуются старые.
    stale: Vec<IVec2>,
    world: World,
    terrain: Terrain,
    seed: i32,
    loader: Loader,
    /// Сколько секций прошло отсечение по пирамиде видимости в последнем кадре.
    drawn_sections: u32,
    /// `None`, если `window.performance` недоступен: тогда замеры нулевые.
    performance: Option<Performance>,
    generate_time: AverageTime,
    mesh_time: AverageTime,
    depth_view: wgpu::TextureView,
    /// Взгляд игрока. Позицию задаёт `player`: каждый кадр камера ставится в его глаза.
    camera: Camera,
    player: Player,
    /// Ноги до последнего шага физики: камера интерполирует от них к `player.feet`.
    prev_feet: Vec3,
    clock: Clock,
    space_tap: DoubleTap,
    /// Коды (`KeyboardEvent.code`) зажатых сейчас клавиш.
    keys: HashSet<String>,
    last_time: Option<f32>,
    /// Фактический бэкенд адаптера: при предпочтении WebGPU может оказаться GL.
    backend: wgpu::Backend,
}

#[wasm_bindgen]
impl Engine {
    /// Инициализирует графику на canvas.
    ///
    /// `preferred` — `"webgpu"` (без поддержки в браузере тихо откатывается на WebGL2)
    /// или `"webgl2"`. `camera` — `[x, y, z, yaw, pitch, mode]` из [`Engine::camera_state`]
    /// (старые 5 значений без `mode` тоже принимаются, режим тогда — ходьба); без него
    /// игрок встаёт на землю над точкой (0, 0). `seed` — сид мира: один сид — один рельеф.
    ///
    /// Сохранённые правки импортируются после create через [`Engine::import_column`],
    /// до первого `render`.
    pub async fn create(
        canvas: HtmlCanvasElement,
        preferred: &str,
        camera: Option<Vec<f32>>,
        seed: i32,
    ) -> Result<Engine, JsError> {
        console_error_panic_hook::set_once();

        let backends = match preferred {
            "webgpu" => wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            "webgl2" => wgpu::Backends::GL,
            other => return Err(JsError::new(&format!("unknown graphics API: {other}"))),
        };
        let terrain = Terrain::new(seed);
        let (camera, player) = match camera {
            Some(state) => restore_player(&state)
                .ok_or_else(|| JsError::new("camera state must be 5 or 6 finite numbers"))?,
            None => {
                // `surface_height` — верхний твёрдый блок, ноги встают на его верхнюю грань.
                let ground = terrain.surface_height(0, 0) as f32;
                let player = Player::new(Vec3::new(0.5, ground + 1.0, 0.5), Mode::Walking);
                (Camera::spawn(player.eye()), player)
            }
        };

        let width = canvas.width().max(1);
        let height = canvas.height().max(1);

        let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = backends;
        // Проверка нужна до create_surface: тип контекста canvas фиксируется первым
        // getContext, и после неудачного "webgpu" тот же canvas не отдаст "webgl2".
        let instance = wgpu::util::new_instance_with_webgpu_detection(instance_desc).await;
        let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await?;
        let backend = adapter.get_info().backend;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("device"),
                // Лимиты WebGL2 — самый слабый общий знаменатель.
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await?;

        let caps = surface.get_capabilities(&adapter);
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

        let performance = web_sys::window().and_then(|w| w.performance());
        let mut generate_time = AverageTime::default();
        let mut mesh_time = AverageTime::default();
        let mut world = World::new();
        let mut loader = Loader::new(camera_column(&camera));
        let mesh_options = MeshOptions::default();
        let start = loader.warm_up(
            &mut world,
            |key| generate_time.measure(&performance, || terrain.generate(key)),
            START_RADIUS,
        );
        let column_meshes = start
            .into_iter()
            .map(|key| {
                let meshes = mesh_time.measure(&performance, || {
                    upload_column(&device, &world, key, mesh_options)
                });
                (key, meshes)
            })
            .collect();

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

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("main"),
            bind_group_layouts: &[Some(&globals_layout)],
            ..Default::default()
        });
        let outline_pipeline = create_outline_pipeline(&device, &pipeline_layout, render_format);
        let outline_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("outline"),
            size: (OUTLINE_VERTICES * OUTLINE_VERTEX_SIZE) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("main"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::layout())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(render_format.into())],
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
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Ok(Engine {
            surface,
            device,
            queue,
            config,
            render_format,
            pipeline,
            outline_pipeline,
            outline_buffer,
            outline_block: None,
            target: None,
            breaking: Repeat::new(REPEAT_S),
            placing: Repeat::new(REPEAT_S),
            selected: DEFAULT_BLOCK,
            globals_buffer,
            globals_bind_group,
            column_meshes,
            mesh_options,
            stale: Vec::new(),
            world,
            terrain,
            seed,
            loader,
            drawn_sections: 0,
            performance,
            generate_time,
            mesh_time,
            depth_view,
            camera,
            prev_feet: player.feet,
            player,
            clock: Clock::default(),
            space_tap: DoubleTap::new(DOUBLE_TAP_S),
            keys: HashSet::new(),
            last_time: None,
            backend,
        })
    }

    /// Есть ли в браузере рабочий WebGPU (не только `navigator.gpu`, но и адаптер).
    pub async fn webgpu_supported() -> bool {
        wgpu::util::is_browser_webgpu_supported().await
    }

    /// Версия формата сохранения (`save::FORMAT_VERSION`): фронт пишет её в метаданные
    /// мира и не загружает сохранение другой версии.
    pub fn format_version() -> u8 {
        FORMAT_VERSION
    }

    /// Фактический графический API: `"webgpu"` или `"webgl2"`.
    pub fn backend(&self) -> String {
        match self.backend {
            wgpu::Backend::BrowserWebGpu => "webgpu",
            _ => "webgl2",
        }
        .to_owned()
    }

    /// Состояние `[x, y, z, yaw, pitch, mode]` для передачи в новый `Engine`: x, y, z —
    /// глаза игрока, `mode` — 0 ходьба, 1 полёт. Скорость не передаётся.
    pub fn camera_state(&self) -> Vec<f32> {
        let eye = self.player.eye();
        let mode = match self.player.mode {
            Mode::Walking => 0.0,
            Mode::Flying => 1.0,
        };
        vec![
            eye.x,
            eye.y,
            eye.z,
            self.camera.yaw,
            self.camera.pitch,
            mode,
        ]
    }

    pub fn seed(&self) -> i32 {
        self.seed
    }

    /// Плоский список `[x0, z0, x1, z1, …]` колонок, изменённых после прошлого вызова:
    /// их надо сохранить через [`Engine::export_column`].
    pub fn drain_dirty(&mut self) -> Vec<i32> {
        self.world
            .drain_dirty()
            .into_iter()
            .flat_map(|key| key.to_array())
            .collect()
    }

    /// Байты изменённой колонки (формат `save::encode_column`) или `None`, если
    /// колонку не меняли.
    pub fn export_column(&self, x: i32, z: i32) -> Option<Vec<u8>> {
        self.world
            .edited_column(IVec2::new(x, z))
            .map(encode_column)
    }

    /// Импорт сохранённой колонки из [`Engine::export_column`]. Меши её и соседей
    /// перестраиваются сразу, если они уже есть. Ошибка формата — `JsError`, мир не меняется.
    pub fn import_column(&mut self, x: i32, z: i32, bytes: &[u8]) -> Result<(), JsError> {
        let column = decode_column(bytes).map_err(|e| JsError::new(&e.to_string()))?;
        let key = IVec2::new(x, z);
        self.world.import(key, column);
        // Соседей целиком, а не только граничные секции: импорт идёт раз при старте,
        // до первого кадра, и простота тут дороже пары лишних мешей. Диагональные — из-за
        // AO на углу колонки.
        for key in std::iter::once(key).chain(NEIGHBORS.map(|n| key + n)) {
            if self.column_meshes.contains_key(&key) {
                let meshes = upload_column(&self.device, &self.world, key, self.mesh_options);
                self.column_meshes.insert(key, meshes);
            }
        }
        Ok(())
    }

    /// Режим игрока: `"walking"` или `"flying"`.
    pub fn player_mode(&self) -> String {
        match self.player.mode {
            Mode::Walking => "walking",
            Mode::Flying => "flying",
        }
        .to_owned()
    }

    /// Стоял ли игрок на блоке в последнем шаге физики.
    pub fn on_ground(&self) -> bool {
        self.player.on_ground
    }

    /// Дальность прорисовки в колонках; значение зажимается в 4..=16. Меняет круг загрузки,
    /// туман и дальнюю плоскость. Можно вызывать в любой момент, в том числе сразу после create.
    pub fn set_render_distance(&mut self, columns: i32) {
        self.loader.set_radius(columns);
    }

    pub fn render_distance(&self) -> i32 {
        self.loader.radius()
    }

    /// Включает или выключает затенение углов (AO). Меши всех колонок перестраиваются
    /// постепенно, ближние первыми, в бюджете подгрузки; до замены рисуются старые.
    pub fn set_ambient_occlusion(&mut self, enabled: bool) {
        if enabled == self.mesh_options.ambient_occlusion {
            return;
        }
        self.mesh_options.ambient_occlusion = enabled;
        let center = camera_column(&self.camera);
        self.stale = self.column_meshes.keys().copied().collect();
        self.stale
            .sort_by_key(|&key| std::cmp::Reverse((key - center).length_squared()));
    }

    /// Включено ли затенение углов; по умолчанию — да.
    pub fn ambient_occlusion(&self) -> bool {
        self.mesh_options.ambient_occlusion
    }

    /// Статистика для HUD: `[колонок загружено, колонок с мешами, секций с мешами,
    /// колонок в очереди на меш (включая перестройку после смены AO), генерация колонки
    /// в мкс, меширование колонки в мкс, секций нарисовано в прошлом кадре]`.
    /// Время — среднее с прошлого вызова (или последнее известное, если работы не было).
    pub fn stats(&mut self) -> Vec<u32> {
        let sections = self.column_meshes.values().flatten().flatten().count();
        vec![
            self.world.len() as u32,
            self.column_meshes.len() as u32,
            sections as u32,
            (self.loader.pending() + self.stale.len()) as u32,
            self.generate_time.take_micros(),
            self.mesh_time.take_micros(),
            self.drawn_sections,
        ]
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

    /// Поворот камеры мышью; `dx`/`dy` — `movementX/Y` при pointer lock.
    pub fn mouse_move(&mut self, dx: f32, dy: f32) {
        self.camera.rotate(dx, dy);
    }

    /// Нажатие/отпускание клавиши; `code` — `KeyboardEvent.code` ("KeyW", "Space"…).
    /// Двойной Space переключает ходьбу и полёт.
    pub fn set_key(&mut self, code: &str, pressed: bool) {
        if pressed {
            // Повтор нажатия без отпускания (автоповтор клавиатуры) — не новое нажатие.
            let fresh = self.keys.insert(code.to_owned());
            // Время прошлого кадра, а не события: точность в кадр для окна 0.3 с хватает.
            if fresh && code == "Space" && self.space_tap.press(self.last_time.unwrap_or(0.0)) {
                self.player.toggle_mode();
            }
        } else {
            self.keys.remove(code);
        }
    }

    /// Кнопка мыши: `button` — `MouseEvent.button` (0 — левая, 2 — правая), остальные
    /// игнорируются. Левая ломает блок под прицелом, правая ставит выбранный; при
    /// удержании действие повторяется раз в [`REPEAT_S`].
    pub fn mouse_button(&mut self, button: u8, pressed: bool) {
        match button {
            0 => self.breaking.set(pressed),
            2 => self.placing.set(pressed),
            _ => {}
        }
    }

    /// Блок для установки: `"grass"`, `"dirt"` или `"stone"`; иначе ошибка.
    pub fn select_block(&mut self, name: &str) -> Result<(), JsError> {
        self.selected = Block::from_name(name)
            .ok_or_else(|| JsError::new(&format!("unknown block: {name}")))?;
        Ok(())
    }

    /// Отпускает все клавиши и кнопки мыши (потеря pointer lock, blur окна).
    pub fn release_all_keys(&mut self) {
        self.keys.clear();
        self.breaking.reset();
        self.placing.reset();
    }

    /// Рисует один кадр. `time` — секунды с начала работы страницы.
    pub fn render(&mut self, time: f32) -> Result<(), JsError> {
        let dt = self
            .last_time
            .map_or(0.0, |last| (time - last).clamp(0.0, MAX_DT));
        self.last_time = Some(time);
        self.update_player(dt);
        self.stream_columns();
        self.interact(time);
        if self.target != self.outline_block {
            if let Some(block) = self.target {
                self.queue
                    .write_buffer(&self.outline_buffer, 0, &outline_bytes(block));
            }
            self.outline_block = self.target;
        }

        let aspect = self.config.width as f32 / self.config.height as f32;
        let fog = Fog::for_radius(self.loader.radius());
        let view_proj = self
            .camera
            .view_proj(aspect, fog.far_plane(self.camera.position.y));
        let globals = Globals {
            view_proj,
            camera_pos: self.camera.position,
            fog_start: fog.start,
            fog_color: Vec3::new(SKY.r as f32, SKY.g as f32, SKY.b as f32),
            fog_end: fog.end,
        };
        self.queue
            .write_buffer(&self.globals_buffer, 0, &globals.to_bytes());
        let frustum = Frustum::from_view_proj(view_proj);

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.surface.configure(&self.device, &self.config);
                frame
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            // Пропускаем кадр и пробуем на следующем.
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            other => return Err(JsError::new(&format!("surface error: {other:?}"))),
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
                        load: wgpu::LoadOp::Clear(SKY),
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
            pass.set_bind_group(0, &self.globals_bind_group, &[]);
            let visible = self
                .column_meshes
                .values()
                .flatten()
                .flatten()
                .filter(|mesh| frustum.intersects_aabb(mesh.min, mesh.max));
            let mut drawn = 0;
            for mesh in visible {
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                drawn += 1;
            }
            self.drawn_sections = drawn;

            // После рельефа: глубина `LessEqual` без записи прячет рёбра за блоками.
            if self.target.is_some() {
                pass.set_pipeline(&self.outline_pipeline);
                pass.set_vertex_buffer(0, self.outline_buffer.slice(..));
                pass.draw(0..OUTLINE_VERTICES as u32, 0..1);
            }
        }
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        Ok(())
    }
}

impl Engine {
    /// Подгрузка колонок вокруг камеры и перестройка устаревших мешей (`stale`), пока
    /// не выйдет [`STREAM_BUDGET_MS`]. Одна операция за кадр делается всегда, даже если
    /// бюджет съели раньше: иначе на медленной машине мир не загрузился бы никогда.
    fn stream_columns(&mut self) {
        let deadline = self
            .performance
            .as_ref()
            .map(|p| p.now() + STREAM_BUDGET_MS);
        // Буферы освобождаются вместе с `SectionMesh`.
        for key in self
            .loader
            .update(camera_column(&self.camera), &mut self.world)
        {
            self.column_meshes.remove(&key);
        }
        let mut loader_done = false;
        for step in 0usize.. {
            // Перестройка чередуется с подгрузкой: при смене AO на ходу у игрока не
            // остаётся дыр на краю мира, а подгрузка не задерживает перестройку.
            if !self.stale.is_empty() && (loader_done || step % 2 == 0) {
                self.rebuild_stale();
            } else if !self.advance_loader() {
                loader_done = true;
                if self.stale.is_empty() {
                    break;
                }
                continue;
            }
            // Без `performance` (в браузере он есть всегда) время не измерить:
            // остаёмся на минимуме, одна операция за кадр.
            let in_time = match (&self.performance, deadline) {
                (Some(p), Some(deadline)) => p.now() < deadline,
                _ => false,
            };
            if !in_time {
                break;
            }
        }
    }

    /// Один шаг загрузчика; `false`, если работы нет.
    fn advance_loader(&mut self) -> bool {
        let progress = self.loader.advance(&mut self.world, |key| {
            self.generate_time
                .measure(&self.performance, || self.terrain.generate(key))
        });
        match progress {
            Progress::Done => return false,
            Progress::Generated(_) => {}
            Progress::Mesh(key) => {
                let meshes = self.mesh_time.measure(&self.performance, || {
                    upload_column(&self.device, &self.world, key, self.mesh_options)
                });
                self.column_meshes.insert(key, meshes);
            }
        }
        true
    }

    /// Перестраивает ближайшую колонку из `stale` с текущими `mesh_options`. Колонку,
    /// чей меш загрузчик уже снял, пропускает.
    fn rebuild_stale(&mut self) {
        let Some(key) = self.stale.pop() else {
            return;
        };
        if self.column_meshes.contains_key(&key) {
            let meshes = self.mesh_time.measure(&self.performance, || {
                upload_column(&self.device, &self.world, key, self.mesh_options)
            });
            self.column_meshes.insert(key, meshes);
        }
    }
}

impl Engine {
    /// Блок под прицелом: луч из камеры вдоль взгляда не дальше [`REACH`].
    fn pick(&self) -> Option<Hit> {
        raycast(self.camera.position, self.camera.forward(), REACH, |pos| {
            self.world.block(pos).is_solid()
        })
    }

    /// Ломание и установка по кнопкам мыши, затем перестройка задетых секций в этом
    /// же кадре — вне бюджета подгрузки, чтобы правка была видна сразу.
    fn interact(&mut self, time: f32) {
        let mut target = self.pick();
        let mut dirty = HashSet::new();
        // `fire` вызывается каждый кадр, даже без цели: так при удержании темп не сбивается.
        if self.breaking.fire(time)
            && let Some(hit) = target
            // Ниже y = 0 — дно мира: `World::block` считает его камнем, и луч в него
            // попадает, но ломать его нельзя.
            && hit.block.y >= 0
            && self.world.set_block(hit.block, Block::Air)
        {
            dirty.extend(affected_sections(hit.block));
            target = self.pick();
        }
        if self.placing.fire(time)
            && let Some(hit) = target
        {
            let pos = hit.block + hit.normal;
            // `set_block` сам отказывает вне высоты мира и в незагруженной колонке.
            if !self.world.block(pos).is_solid()
                && !self.player.overlaps_block(pos)
                && self.world.set_block(pos, self.selected)
            {
                dirty.extend(affected_sections(pos));
                target = self.pick();
            }
        }
        for key in dirty {
            self.remesh_section(key);
        }
        self.target = target.map(|hit| hit.block);
    }

    /// Перестраивает меш секции `(x колонки, номер секции, z колонки)` по текущему миру.
    /// Колонку без меша пропускает: загрузчик мешует её целиком позже, уже с правкой.
    fn remesh_section(&mut self, key: IVec3) {
        let Some(meshes) = self.column_meshes.get_mut(&IVec2::new(key.x, key.z)) else {
            return;
        };
        let Some(data) = mesh_world_section(&self.world, key, self.mesh_options) else {
            return;
        };
        let min = (key * SECTION_SIZE).as_vec3();
        meshes[key.y as usize] = upload_section(&self.device, &data, min);
    }
}

impl Engine {
    /// Шаги физики за время кадра `dt` и камера в интерполированных глазах игрока.
    fn update_player(&mut self, dt: f32) {
        let input = self.input();
        for _ in 0..self.clock.advance(dt) {
            if !self.body_loaded() {
                // Под незагруженной колонкой `World::block` — воздух, и игрок провалился
                // бы сквозь ещё не сгенерированную землю. Стоим, пока она не появится;
                // скорость сохраняется, чтобы падение продолжилось как было.
                self.clock.reset();
                self.prev_feet = self.player.feet;
                break;
            }
            self.prev_feet = self.player.feet;
            self.player.step(&input, self.camera.yaw, |pos| {
                self.world.block(pos).is_solid()
            });
        }
        let feet = self.prev_feet.lerp(self.player.feet, self.clock.alpha());
        self.camera.position = feet + Vec3::Y * EYE_HEIGHT;
    }

    /// Намерения игрока по зажатым клавишам.
    fn input(&self) -> Input {
        let pressed = |code: &str| self.keys.contains(code);
        let axis = |plus: &str, minus: &str| {
            f32::from(u8::from(pressed(plus))) - f32::from(u8::from(pressed(minus)))
        };
        Input {
            forward: axis("KeyW", "KeyS"),
            strafe: axis("KeyD", "KeyA"),
            jump: pressed("Space"),
            descend: pressed("ShiftLeft") || pressed("ShiftRight"),
        }
    }

    /// Загружены ли все колонки, которые тело может задеть за шаг. Запас в блок по
    /// горизонтали с лихвой покрывает шаг полёта (8 / 60 блока).
    fn body_loaded(&self) -> bool {
        let (min, max) = self.player.aabb();
        let lo = (min - Vec3::ONE).floor().as_ivec3();
        let hi = (max + Vec3::ONE).floor().as_ivec3();
        let (lo, hi) = (column_key(lo.x, lo.z), column_key(hi.x, hi.z));
        (lo.x..=hi.x).all(|x| (lo.y..=hi.y).all(|z| self.world.column(IVec2::new(x, z)).is_some()))
    }
}

/// Камера и игрок из [`Engine::camera_state`]: `[x, y, z, yaw, pitch]` и необязательный
/// `mode`. `None`, если длина другая или есть не-числа.
fn restore_player(state: &[f32]) -> Option<(Camera, Player)> {
    let (view, mode) = match state {
        [view @ .., mode] if state.len() == 6 => (view, Some(*mode)),
        _ => (state, None),
    };
    let camera = Camera::from_state(view)?;
    let mode = match mode {
        None => Mode::Walking,
        Some(1.0) => Mode::Flying,
        Some(m) if m.is_finite() => Mode::Walking,
        Some(_) => return None,
    };
    let player = Player::new(camera.position - Vec3::Y * EYE_HEIGHT, mode);
    Some((camera, player))
}

/// Ключ колонки, над которой стоит камера.
fn camera_column(camera: &Camera) -> IVec2 {
    let p = camera.position.floor().as_ivec3();
    column_key(p.x, p.z)
}

/// Среднее время одной операции между вызовами [`AverageTime::take_micros`].
#[derive(Default)]
struct AverageTime {
    total_ms: f64,
    count: u32,
    last_micros: u32,
}

impl AverageTime {
    fn measure<T>(&mut self, performance: &Option<Performance>, f: impl FnOnce() -> T) -> T {
        let Some(performance) = performance else {
            return f();
        };
        let start = performance.now();
        let result = f();
        self.total_ms += performance.now() - start;
        self.count += 1;
        result
    }

    fn take_micros(&mut self) -> u32 {
        if self.count > 0 {
            self.last_micros = (self.total_ms * 1000.0 / f64::from(self.count)) as u32;
            self.total_ms = 0.0;
            self.count = 0;
        }
        self.last_micros
    }
}

/// GPU-буферы меша одной секции. Индексы `u16`: на секцию их всегда хватает.
struct SectionMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    /// Коробка секции целиком в мировых координатах — для отсечения по пирамиде.
    min: Vec3,
    max: Vec3,
}

/// Меши секций колонки снизу вверх; `None` — секция без видимых граней. Номер
/// секции нужен, чтобы после правки блока перестроить одну её, а не всю колонку.
type ColumnMeshes = [Option<SectionMesh>; COLUMN_SECTIONS as usize];

/// Строит меш колонки и загружает её непустые секции на GPU. Загрузчик отдаёт
/// колонку на меш только вместе с 8 соседями, так что грани и AO на стыках верные сразу.
fn upload_column(
    device: &wgpu::Device,
    world: &World,
    key: IVec2,
    options: MeshOptions,
) -> ColumnMeshes {
    let column = world
        .column(key)
        .expect("loader meshes only loaded columns");
    let origin = IVec3::new(key.x * SECTION_SIZE, 0, key.y * SECTION_SIZE);
    let data = mesh_column(column, origin, |local| world.block(origin + local), options);
    std::array::from_fn(|sy| {
        let min = (origin + IVec3::Y * sy as i32 * SECTION_SIZE).as_vec3();
        upload_section(device, &data[sy], min)
    })
}

/// `min` — нижний угол секции в мире. `None` для пустой секции: буфер нулевого
/// размера не нужен.
fn upload_section(device: &wgpu::Device, data: &MeshData, min: Vec3) -> Option<SectionMesh> {
    if data.indices.is_empty() {
        return None;
    }
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("section vertices"),
        contents: &mesh::vertex_bytes(&data.vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("section indices"),
        contents: &mesh::index_bytes(&data.indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    Some(SectionMesh {
        vertex_buffer,
        index_buffer,
        index_count: data.indices.len() as u32,
        min,
        max: min + SECTION_SIZE as f32,
    })
}

/// Пайплайн рамки вокруг блока под прицелом: линии поверх рельефа с тем же буфером глубины.
fn create_outline_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("outline.wgsl"));
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

/// Массив текстур блоков: по слою на тайл, с готовыми мип-уровнями.
fn create_block_textures(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::TextureView {
    let texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("blocks"),
            size: wgpu::Extent3d {
                width: TILE_SIZE,
                height: TILE_SIZE,
                depth_or_array_layers: LAYER_COUNT,
            },
            mip_level_count: MIP_LEVELS,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &texture::generate(),
    );
    texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    })
}

fn create_depth_view(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
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
