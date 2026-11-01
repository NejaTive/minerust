use game::Game;
use render::{Effects, Renderer};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use world::block::Block;
use world::clouds::CloudMask;
use world::pos::ColumnPos;
use world::save::{FORMAT_VERSION, decode_column, encode_column};
use world::terrain::WorldType;

use crate::api::{EngineSettings, WorldStats};
use crate::frame::frame_params;
use crate::profiler::{Profiler, SceneStats};
use crate::streamer::Streamer;

const BAD_CAMERA_STATE: &str = "camera state must be 5 or 6 finite numbers";

#[wasm_bindgen]
pub struct Engine {
    game: Game,
    renderer: Renderer,
    streamer: Streamer,
    profiler: Profiler,
    clouds: bool,
    effects: Effects,
}

#[wasm_bindgen]
impl Engine {
    pub async fn create(
        canvas: HtmlCanvasElement,
        camera: Option<Vec<f32>>,
        seed: i32,
        world_type: &str,
    ) -> Result<Engine, JsError> {
        console_error_panic_hook::set_once();

        let world_type = WorldType::from_name(world_type)
            .ok_or_else(|| JsError::new(&format!("unknown world type: {world_type}")))?;
        let mut game = Game::of(seed, world_type, camera.as_deref())
            .map_err(|_| JsError::new(BAD_CAMERA_STATE))?;
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
        renderer.set_cloud_mask(&CloudMask::new(seed));

        let mut profiler = Profiler::new();
        let mut streamer = Streamer::new(game.camera_column());
        streamer.warm_up(&mut game, &mut renderer, &mut profiler);
        Ok(Engine {
            game,
            renderer,
            streamer,
            profiler,
            clouds: true,
            effects: Effects::default(),
        })
    }

    pub fn format_version() -> u8 {
        FORMAT_VERSION
    }

    pub fn camera_state(&self) -> Vec<f32> {
        self.game.camera_state()
    }

    pub fn seed(&self) -> i32 {
        self.game.seed()
    }

    pub fn drain_dirty(&mut self) -> Vec<i32> {
        self.game
            .world
            .drain_dirty()
            .into_iter()
            .flat_map(|key| [key.x(), key.z()])
            .collect()
    }

    pub fn export_column(&self, x: i32, z: i32) -> Option<Vec<u8>> {
        self.game
            .world
            .edited_column(ColumnPos::new(x, z))
            .map(encode_column)
    }

    pub fn import_column(&mut self, x: i32, z: i32, bytes: &[u8]) -> Result<(), JsError> {
        let column = decode_column(bytes).map_err(|e| JsError::new(&e.to_string()))?;
        let key = ColumnPos::new(x, z);
        self.game.import_column(key, column);
        self.streamer
            .remesh_around(&self.game, &mut self.renderer, key);
        Ok(())
    }

    pub fn player_mode(&self) -> String {
        use game::physics::Mode;
        match self.game.player().mode {
            Mode::Walking => "walking",
            Mode::Flying => "flying",
            Mode::Noclip => "noclip",
        }
        .to_owned()
    }

    pub fn on_ground(&self) -> bool {
        self.game.player().on_ground
    }

    pub fn apply_settings(&mut self, settings: &EngineSettings) {
        self.streamer.set_radius(settings.render_distance);
        self.game.set_view_radius(settings.render_distance);
        self.streamer
            .set_ambient_occlusion(settings.ambient_occlusion, &self.game, &self.renderer);
        self.clouds = settings.clouds;
        self.effects = Effects {
            colored_light: settings.colored_light,
            water_reflections: settings.water_reflections,
            ground_fog: settings.ground_fog,
            plant_sway: settings.plant_sway,
        };
        self.game.set_weather_enabled(settings.weather);
        self.game.set_auto_step(settings.auto_step);
        self.game
            .camera_mut()
            .set_sensitivity(settings.mouse_sensitivity);
        self.game.camera_mut().set_fov(settings.fov);
    }

    pub fn stats(&mut self) -> WorldStats {
        let scene = SceneStats {
            loaded_columns: self.game.world.len(),
            meshed_columns: self.renderer.column_count(),
            sections: self.renderer.section_count(),
            pending: self.streamer.pending(),
            mesh_jobs: self.streamer.jobs_in_flight(),
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

    pub fn set_mesh_slots(&mut self, slots: u32) {
        self.streamer.set_job_slots(slots as usize);
    }

    pub fn take_mesh_job(&mut self) -> Option<Vec<u8>> {
        self.streamer.take_job()
    }

    pub fn submit_mesh_result(&mut self, bytes: &[u8], mesh_ms: f64) {
        self.streamer
            .push_result(bytes, mesh_ms, &mut self.profiler);
    }

    pub fn load_ms(&self) -> u32 {
        self.profiler.load_ms()
    }

    pub fn set_profiling(&mut self, enabled: bool) {
        self.profiler.set_profiling(enabled);
        self.renderer.set_gpu_timing(enabled);
    }

    pub fn set_camera_state(&mut self, state: Vec<f32>) -> Result<(), JsError> {
        self.game
            .set_camera_state(&state)
            .map_err(|_| JsError::new(BAD_CAMERA_STATE))?;
        self.profiler.restart_load();
        Ok(())
    }

    pub fn build_profile() -> String {
        if cfg!(debug_assertions) {
            "dev"
        } else {
            "release"
        }
        .to_owned()
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.game.set_paused(paused);
    }

    pub fn time_of_day(&self) -> f32 {
        self.game.time_of_day()
    }

    pub fn set_time_of_day(&mut self, time: f32) {
        self.game.set_time_of_day(time);
    }

    pub fn day_phase(&self) -> String {
        self.game.day_phase().to_owned()
    }

    pub fn set_time_frozen(&mut self, frozen: bool) {
        self.game.set_time_frozen(frozen);
    }

    pub fn set_time_fast(&mut self, fast: bool) {
        self.game.set_time_fast(fast);
    }

    pub fn cycle_weather(&mut self) {
        self.game.cycle_weather();
    }

    pub fn weather(&self) -> String {
        self.game.weather().name().to_owned()
    }

    pub fn weather_state(&self) -> Vec<f32> {
        self.game.weather_state()
    }

    pub fn set_weather_state(&mut self, state: Vec<f32>) {
        self.game.set_weather_state(&state);
    }

    pub fn mouse_move(&mut self, dx: f32, dy: f32) {
        self.game.mouse_move(dx, dy);
    }

    pub fn set_key(&mut self, code: &str, pressed: bool) {
        self.game.set_key(code, pressed);
    }

    pub fn toggle_noclip(&mut self) {
        self.game.toggle_noclip();
    }

    pub fn mouse_button(&mut self, button: u8, pressed: bool) {
        self.game.mouse_button(button, pressed);
    }

    pub fn select_block(&mut self, name: &str) -> Result<(), JsError> {
        if self.game.select_block(name) {
            Ok(())
        } else {
            Err(JsError::new(&format!("unknown block: {name}")))
        }
    }

    pub fn release_all_keys(&mut self) {
        self.game.release_all_keys();
    }

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
        if let Some(heights) = self.game.take_rain_cover() {
            self.renderer.set_rain_cover(heights);
        }

        let params = frame_params(
            &self.game,
            self.streamer.radius(),
            self.renderer.aspect(),
            self.clouds,
            self.effects,
        );
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

#[wasm_bindgen]
pub fn block_icon(name: &str) -> Option<Vec<u8>> {
    Block::from_name(name)?.icon_pixels()
}
