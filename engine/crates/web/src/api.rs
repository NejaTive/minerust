use game::camera::{
    DEFAULT_FOV, DEFAULT_SENSITIVITY, MAX_FOV, MAX_SENSITIVITY, MIN_FOV, MIN_SENSITIVITY,
};
use game::loader::{DEFAULT_RADIUS, MAX_RADIUS, MIN_RADIUS};
use mesh::mesher::MeshOptions;
use wasm_bindgen::prelude::*;
use world::block::BLOCKS;

#[wasm_bindgen]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WorldStats {
    pub columns: u32,
    #[wasm_bindgen(js_name = meshedColumns)]
    pub meshed_columns: u32,
    #[wasm_bindgen(js_name = meshedSections)]
    pub meshed_sections: u32,
    pub queued: u32,
    #[wasm_bindgen(js_name = meshJobs)]
    pub mesh_jobs: u32,
    #[wasm_bindgen(js_name = encodeUs)]
    pub encode_us: u32,
    #[wasm_bindgen(js_name = applyUs)]
    pub apply_us: u32,
    #[wasm_bindgen(js_name = workerMeshUs)]
    pub worker_mesh_us: u32,
    #[wasm_bindgen(js_name = generateUs)]
    pub generate_us: u32,
    #[wasm_bindgen(js_name = meshUs)]
    pub mesh_us: u32,
    #[wasm_bindgen(js_name = drawnSections)]
    pub drawn_sections: u32,
    #[wasm_bindgen(js_name = lightUs)]
    pub light_us: u32,
    #[wasm_bindgen(js_name = editUs)]
    pub edit_us: u32,
    #[wasm_bindgen(js_name = lightKib)]
    pub light_kib: u32,
    #[wasm_bindgen(js_name = streamUs)]
    pub stream_us: u32,
    #[wasm_bindgen(js_name = buildUs)]
    pub build_us: u32,
    #[wasm_bindgen(js_name = submitUs)]
    pub submit_us: u32,
    #[wasm_bindgen(js_name = solidQuads)]
    pub solid_quads: u32,
    #[wasm_bindgen(js_name = cutoutQuads)]
    pub cutout_quads: u32,
    #[wasm_bindgen(js_name = waterQuads)]
    pub water_quads: u32,
    #[wasm_bindgen(js_name = drawCalls)]
    pub draw_calls: u32,
    #[wasm_bindgen(js_name = gpuKib)]
    pub gpu_kib: u32,
    #[wasm_bindgen(js_name = gpuUsedKib)]
    pub gpu_used_kib: u32,
    #[wasm_bindgen(js_name = gpuFrameUs)]
    pub gpu_frame_us: u32,
    #[wasm_bindgen(js_name = gpuTimer)]
    pub gpu_timer: bool,
    #[wasm_bindgen(js_name = loadMs)]
    pub load_ms: u32,
}

#[wasm_bindgen]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockInfo {
    #[wasm_bindgen(getter_with_clone)]
    pub name: String,
    #[wasm_bindgen(getter_with_clone)]
    pub label: String,
    pub placeable: bool,
}

#[wasm_bindgen]
pub fn blocks() -> Vec<BlockInfo> {
    BLOCKS
        .iter()
        .map(|def| BlockInfo {
            name: def.name.to_owned(),
            label: def.label.to_owned(),
            placeable: def.selectable,
        })
        .collect()
}

#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EngineSettings {
    #[wasm_bindgen(js_name = renderDistance)]
    pub render_distance: i32,
    #[wasm_bindgen(js_name = ambientOcclusion)]
    pub ambient_occlusion: bool,
    pub clouds: bool,
    pub weather: bool,
    #[wasm_bindgen(js_name = autoStep)]
    pub auto_step: bool,
    #[wasm_bindgen(js_name = mouseSensitivity)]
    pub mouse_sensitivity: f32,
    pub fov: f32,
    #[wasm_bindgen(js_name = coloredLight)]
    pub colored_light: bool,
    #[wasm_bindgen(js_name = waterReflections)]
    pub water_reflections: bool,
    #[wasm_bindgen(js_name = groundFog)]
    pub ground_fog: bool,
    #[wasm_bindgen(js_name = plantSway)]
    pub plant_sway: bool,
    pub hdr: bool,
}

#[wasm_bindgen]
impl EngineSettings {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        render_distance: i32,
        ambient_occlusion: bool,
        clouds: bool,
        weather: bool,
        auto_step: bool,
        mouse_sensitivity: f32,
        fov: f32,
        colored_light: bool,
        water_reflections: bool,
        ground_fog: bool,
        plant_sway: bool,
        hdr: bool,
    ) -> EngineSettings {
        EngineSettings {
            render_distance,
            ambient_occlusion,
            clouds,
            weather,
            auto_step,
            mouse_sensitivity,
            fov,
            colored_light,
            water_reflections,
            ground_fog,
            plant_sway,
            hdr,
        }
    }
}

#[wasm_bindgen]
pub fn default_settings() -> EngineSettings {
    EngineSettings {
        render_distance: DEFAULT_RADIUS,
        ambient_occlusion: MeshOptions::default().ambient_occlusion,
        clouds: true,
        weather: true,
        auto_step: true,
        mouse_sensitivity: DEFAULT_SENSITIVITY,
        fov: DEFAULT_FOV,
        colored_light: true,
        water_reflections: true,
        ground_fog: true,
        plant_sway: true,
        hdr: true,
    }
}

#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SettingsLimits {
    #[wasm_bindgen(js_name = renderDistanceMin)]
    pub render_distance_min: i32,
    #[wasm_bindgen(js_name = renderDistanceMax)]
    pub render_distance_max: i32,
    #[wasm_bindgen(js_name = mouseSensitivityMin)]
    pub mouse_sensitivity_min: f32,
    #[wasm_bindgen(js_name = mouseSensitivityMax)]
    pub mouse_sensitivity_max: f32,
    #[wasm_bindgen(js_name = fovMin)]
    pub fov_min: f32,
    #[wasm_bindgen(js_name = fovMax)]
    pub fov_max: f32,
}

#[wasm_bindgen]
pub fn settings_limits() -> SettingsLimits {
    SettingsLimits {
        render_distance_min: MIN_RADIUS,
        render_distance_max: MAX_RADIUS,
        mouse_sensitivity_min: MIN_SENSITIVITY,
        mouse_sensitivity_max: MAX_SENSITIVITY,
        fov_min: MIN_FOV,
        fov_max: MAX_FOV,
    }
}

#[cfg(test)]
mod tests;
