use game::Game;
use render::Renderer;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use world::block::Block;
use world::pos::ColumnPos;
use world::save::{FORMAT_VERSION, decode_column, encode_column};

use crate::api::{EngineSettings, WorldStats};
use crate::frame::frame_params;
use crate::profiler::{Profiler, SceneStats};
use crate::streamer::Streamer;

const BAD_CAMERA_STATE: &str = "camera state must be 5 or 6 finite numbers";

/// Движок игры: склейка `Game` (симуляция), `Renderer` (GPU), `Streamer` (подгрузка) и
/// `Profiler` (замеры); рисует кадры на переданном canvas.
#[wasm_bindgen]
pub struct Engine {
    game: Game,
    renderer: Renderer,
    streamer: Streamer,
    profiler: Profiler,
}

#[wasm_bindgen]
impl Engine {
    /// Инициализирует графику на canvas.
    ///
    /// Нужен браузер с WebGPU: без адаптера или устройства возвращает ошибку.
    /// `camera` — `[x, y, z, yaw, pitch, mode]` из [`Engine::camera_state`]
    /// (старые 5 значений без `mode` тоже принимаются, режим тогда — ходьба); без него
    /// игрок встаёт на землю над точкой (0, 0). `seed` — сид мира: один сид — один рельеф.
    ///
    /// Сохранённые правки импортируются после create через [`Engine::import_column`],
    /// до первого `render`.
    pub async fn create(
        canvas: HtmlCanvasElement,
        camera: Option<Vec<f32>>,
        seed: i32,
    ) -> Result<Engine, JsError> {
        console_error_panic_hook::set_once();

        let mut game =
            Game::new(seed, camera.as_deref()).map_err(|_| JsError::new(BAD_CAMERA_STATE))?;
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);

        let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = wgpu::Backends::BROWSER_WEBGPU;
        let instance = wgpu::Instance::new(instance_desc);
        let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(|e| JsError::new(&format!("WebGPU adapter is unavailable: {e}")))?;
        let mut renderer = Renderer::new(surface, &adapter, width, height)
            .await
            .map_err(|e| JsError::new(&e))?;

        let mut profiler = Profiler::new();
        let mut streamer = Streamer::new(game.camera_column());
        streamer.warm_up(&mut game, &mut renderer, &mut profiler);
        Ok(Engine {
            game,
            renderer,
            streamer,
            profiler,
        })
    }

    /// Версия формата сохранения (`save::FORMAT_VERSION`): фронт пишет её в метаданные
    /// мира и не загружает сохранение другой версии.
    pub fn format_version() -> u8 {
        FORMAT_VERSION
    }

    /// Состояние `[x, y, z, yaw, pitch, mode]` для передачи в новый `Engine`: x, y, z —
    /// глаза игрока, `mode` — 0 ходьба, 1 полёт, 2 полёт сквозь блоки. Скорость не
    /// передаётся.
    pub fn camera_state(&self) -> Vec<f32> {
        self.game.camera_state()
    }

    pub fn seed(&self) -> i32 {
        self.game.seed()
    }

    /// Плоский список `[x0, z0, x1, z1, …]` колонок, изменённых после прошлого вызова:
    /// их надо сохранить через [`Engine::export_column`].
    pub fn drain_dirty(&mut self) -> Vec<i32> {
        self.game
            .world
            .drain_dirty()
            .into_iter()
            .flat_map(|key| [key.x(), key.z()])
            .collect()
    }

    /// Байты изменённой колонки (формат `save::encode_column`) или `None`, если
    /// колонку не меняли.
    pub fn export_column(&self, x: i32, z: i32) -> Option<Vec<u8>> {
        self.game
            .world
            .edited_column(ColumnPos::new(x, z))
            .map(encode_column)
    }

    /// Импорт сохранённой колонки из [`Engine::export_column`]. Меши её и соседей
    /// перестраиваются сразу, если они уже есть. Ошибка формата — `JsError`, мир не меняется.
    pub fn import_column(&mut self, x: i32, z: i32, bytes: &[u8]) -> Result<(), JsError> {
        let column = decode_column(bytes).map_err(|e| JsError::new(&e.to_string()))?;
        let key = ColumnPos::new(x, z);
        self.game.import_column(key, column);
        self.streamer
            .remesh_around(&self.game, &mut self.renderer, key);
        Ok(())
    }

    /// Режим игрока: `"walking"`, `"flying"` или `"noclip"`.
    pub fn player_mode(&self) -> String {
        use game::physics::Mode;
        match self.game.player().mode {
            Mode::Walking => "walking",
            Mode::Flying => "flying",
            Mode::Noclip => "noclip",
        }
        .to_owned()
    }

    /// Стоял ли игрок на блоке в последнем шаге физики.
    pub fn on_ground(&self) -> bool {
        self.game.player().on_ground
    }

    /// Применяет настройки одним объектом; значения зажимаются в пределы
    /// [`settings_limits`](crate::api::settings_limits). Идемпотентна, вызывается в любой
    /// момент, в том числе сразу после create. Смена AO перестраивает меши всех колонок
    /// постепенно, ближние первыми, в бюджете подгрузки; до замены рисуются старые.
    pub fn apply_settings(&mut self, settings: &EngineSettings) {
        self.streamer.set_radius(settings.render_distance);
        self.streamer
            .set_ambient_occlusion(settings.ambient_occlusion, &self.game, &self.renderer);
        self.game
            .camera_mut()
            .set_sensitivity(settings.mouse_sensitivity);
        self.game.camera_mut().set_fov(settings.fov);
    }

    /// Статистика для HUD и режима замера; поля и единицы — в [`WorldStats`].
    pub fn stats(&mut self) -> WorldStats {
        let scene = SceneStats {
            loaded_columns: self.game.world.len(),
            meshed_columns: self.renderer.column_count(),
            sections: self.renderer.section_count(),
            pending: self.streamer.pending(),
            drawn_sections: self.renderer.drawn_sections(),
            light_kib: self.game.light.memory_bytes() / 1024,
            counts: self.renderer.frame_counts(),
            gpu_kib: self.renderer.gpu_bytes() / 1024,
            gpu_used_kib: self.renderer.gpu_used_bytes() / 1024,
            gpu_timer: self.renderer.has_gpu_timer(),
        };
        if let Some((total_ms, count)) = self.renderer.take_gpu_samples() {
            self.profiler.gpu_frame_time.record_total(total_ms, count);
        }
        self.profiler.stats(&scene)
    }

    /// Длительность последней завершённой загрузки мира, мс; 0, пока идёт. В отличие от
    /// `stats()` ничего не сбрасывает, так что режим замера опрашивает её каждый кадр.
    pub fn load_ms(&self) -> u32 {
        self.profiler.load_ms()
    }

    /// Замеры времени кадра для `stats()`: три `performance.now` за кадр и метки времени GPU. Выключены по
    /// умолчанию, включаются отладочным HUD и режимом замера.
    pub fn set_profiling(&mut self, enabled: bool) {
        self.profiler.set_profiling(enabled);
        self.renderer.set_gpu_timing(enabled);
    }

    /// Отладочная: ставит игрока и взгляд из `[x, y, z, yaw, pitch, mode]` (как
    /// [`Engine::camera_state`]) и начинает отсчёт полной загрузки заново. Нужна режиму
    /// замера `?bench`; в обычной игре не вызывается.
    pub fn set_camera_state(&mut self, state: Vec<f32>) -> Result<(), JsError> {
        self.game
            .set_camera_state(&state)
            .map_err(|_| JsError::new(BAD_CAMERA_STATE))?;
        self.profiler.restart_load();
        Ok(())
    }

    /// `"release"` или `"dev"`: профиль, с которым собран wasm (влияет на цифры замеров).
    pub fn build_profile() -> String {
        if cfg!(debug_assertions) {
            "dev"
        } else {
            "release"
        }
        .to_owned()
    }

    /// Вызывается при изменении размеров canvas (в физических пикселях).
    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
    }

    /// Пауза: физика, поворот камеры, ввод и действия мыши стоят; подгрузка колонок и
    /// отрисовка идут. При снятии паузы накопитель физики обнуляется, чтобы игрок не
    /// «догонял» простой. Включение паузы отпускает клавиши и кнопки мыши.
    pub fn set_paused(&mut self, paused: bool) {
        self.game.set_paused(paused);
    }

    /// Время суток 0..1 (0 — начало дня, см. `daytime`).
    pub fn time_of_day(&self) -> f32 {
        self.game.time_of_day()
    }

    /// Ставит время суток; вне 0..1 берётся дробная часть, не число — утро. Меши не
    /// перестраиваются: время меняет только uniform кадра.
    pub fn set_time_of_day(&mut self, time: f32) {
        self.game.set_time_of_day(time);
    }

    /// Фаза суток: `day`, `dusk`, `night` или `dawn`.
    pub fn day_phase(&self) -> String {
        self.game.day_phase().to_owned()
    }

    /// Замораживает время суток (режим замера).
    pub fn set_time_frozen(&mut self, frozen: bool) {
        self.game.set_time_frozen(frozen);
    }

    /// Отладочная перемотка времени суток.
    pub fn set_time_fast(&mut self, fast: bool) {
        self.game.set_time_fast(fast);
    }

    /// Поворот камеры мышью; `dx`/`dy` — `movementX/Y` при pointer lock.
    pub fn mouse_move(&mut self, dx: f32, dy: f32) {
        self.game.mouse_move(dx, dy);
    }

    /// Нажатие/отпускание клавиши; `code` — `KeyboardEvent.code` ("KeyW", "Space"…).
    /// Двойной Space переключает ходьбу и полёт, двойной W включает бег.
    pub fn set_key(&mut self, code: &str, pressed: bool) {
        self.game.set_key(code, pressed);
    }

    /// Включает отладочный полёт сквозь блоки или возвращает из него в обычный полёт.
    /// На паузе не действует, как и клавиши.
    pub fn toggle_noclip(&mut self) {
        self.game.toggle_noclip();
    }

    /// Кнопка мыши: `button` — `MouseEvent.button` (0 — левая, 2 — правая), остальные
    /// игнорируются. Левая ломает блок под прицелом, правая ставит выбранный; при
    /// удержании действие повторяется раз в четверть секунды.
    pub fn mouse_button(&mut self, button: u8, pressed: bool) {
        self.game.mouse_button(button, pressed);
    }

    /// Блок для установки по имени из [`Block::from_name`]; пустая строка — пустая рука,
    /// правая кнопка ничего не ставит. Незнакомое имя — ошибка.
    pub fn select_block(&mut self, name: &str) -> Result<(), JsError> {
        if self.game.select_block(name) {
            Ok(())
        } else {
            Err(JsError::new(&format!("unknown block: {name}")))
        }
    }

    /// Отпускает все клавиши и кнопки мыши (потеря pointer lock, blur окна).
    pub fn release_all_keys(&mut self) {
        self.game.release_all_keys();
    }

    /// Рисует один кадр. `time` — секунды с начала работы страницы.
    pub fn render(&mut self, time: f32) -> Result<(), JsError> {
        let t_start = self.profiler.tick();
        self.game.update(time);
        let t_stream = self.profiler.tick();
        self.streamer
            .update(&mut self.game, &mut self.renderer, &mut self.profiler);
        let t_streamed = self.profiler.tick();
        let edit_started = self.profiler.now();
        let edits = self.game.interact(time);
        self.streamer.apply_edits(
            edits,
            &self.game,
            &mut self.renderer,
            &mut self.profiler,
            edit_started,
        );
        self.renderer.set_outline(self.game.target_block());

        let params = frame_params(&self.game, self.streamer.radius(), self.renderer.aspect());
        let Some(pending) = self.renderer.begin(&params).map_err(|e| JsError::new(&e))? else {
            return Ok(());
        };
        let t_encoded = self.profiler.tick();
        self.renderer.submit(pending);
        let t_end = self.profiler.tick();
        self.profiler
            .record_frame([t_start, t_stream, t_streamed, t_encoded, t_end]);
        Ok(())
    }
}

/// Иконка блока для хотбара: тайл его боковой грани, 16×16 RGBA sRGB, строки сверху вниз.
/// `None` для незнакомого имени (см. [`Block::from_name`]).
#[wasm_bindgen]
pub fn block_icon(name: &str) -> Option<Vec<u8>> {
    Block::from_name(name)?.icon_pixels()
}
