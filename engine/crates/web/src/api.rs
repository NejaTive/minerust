//! Типы JS API, которые не требуют GPU: собираются и тестируются на хосте.
//! Поля — camelCase (`js_name`), как принято в TypeScript; методы `Engine` остаются snake_case.

use game::camera::{
    DEFAULT_FOV, DEFAULT_SENSITIVITY, MAX_FOV, MAX_SENSITIVITY, MIN_FOV, MIN_SENSITIVITY,
};
use game::loader::{DEFAULT_RADIUS, MAX_RADIUS, MIN_RADIUS};
use mesh::mesher::MeshOptions;
use wasm_bindgen::prelude::*;
use world::block::BLOCKS;

/// Статистика для HUD и режима замера (`Engine::stats`). Время — среднее с прошлого вызова
/// (или последнее известное, если работы не было).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WorldStats {
    /// Колонок загружено.
    pub columns: u32,
    /// Колонок с мешами.
    #[wasm_bindgen(js_name = meshedColumns)]
    pub meshed_columns: u32,
    /// Секций с мешами.
    #[wasm_bindgen(js_name = meshedSections)]
    pub meshed_sections: u32,
    /// Колонок в очереди на меш, включая перестройку после смены AO и секции после смены света.
    pub queued: u32,
    /// Генерация колонки, мкс.
    #[wasm_bindgen(js_name = generateUs)]
    pub generate_us: u32,
    /// Меширование колонки, мкс.
    #[wasm_bindgen(js_name = meshUs)]
    pub mesh_us: u32,
    /// Секций, прошедших отсечение по пирамиде видимости, в последнем кадре.
    #[wasm_bindgen(js_name = drawnSections)]
    pub drawn_sections: u32,
    /// Свет колонки при подгрузке, мкс.
    #[wasm_bindgen(js_name = lightUs)]
    pub light_us: u32,
    /// Правка блока со светом и перестройкой, мкс.
    #[wasm_bindgen(js_name = editUs)]
    pub edit_us: u32,
    /// Память на свет, КиБ.
    #[wasm_bindgen(js_name = lightKib)]
    pub light_kib: u32,
    /// CPU за кадр: подгрузка и меш в бюджете, мкс. Нули без `set_profiling(true)`.
    #[wasm_bindgen(js_name = streamUs)]
    pub stream_us: u32,
    /// CPU за кадр: сборка до `finish` (физика, отсечение, запись команд), мкс. Нули без профилирования.
    #[wasm_bindgen(js_name = buildUs)]
    pub build_us: u32,
    /// CPU за кадр: `finish`, `submit`, `present`, мкс. Нули без профилирования.
    #[wasm_bindgen(js_name = submitUs)]
    pub submit_us: u32,
    /// Квады непрозрачных мешей, нарисованные в последнем кадре (вершин в четыре раза больше).
    #[wasm_bindgen(js_name = solidQuads)]
    pub solid_quads: u32,
    /// Квады вырезанных мешей (листва, кресты, факел) в последнем кадре.
    #[wasm_bindgen(js_name = cutoutQuads)]
    pub cutout_quads: u32,
    /// Квады воды в последнем кадре.
    #[wasm_bindgen(js_name = waterQuads)]
    pub water_quads: u32,
    /// Draw-вызовов в последнем кадре.
    #[wasm_bindgen(js_name = drawCalls)]
    pub draw_calls: u32,
    /// Буферы мешей на GPU целиком (арена записей квадов, буфер слотов и индексы квадов, с запасом), КиБ.
    #[wasm_bindgen(js_name = gpuKib)]
    pub gpu_kib: u32,
    /// Из `gpuKib` занято мешами, КиБ; разница — запас арены.
    #[wasm_bindgen(js_name = gpuUsedKib)]
    pub gpu_used_kib: u32,
    /// GPU за кадр (один render pass), мкс. 0 — нет данных: профилирование выключено,
    /// замеры ещё не пришли (результат опаздывает на пару кадров) или нет `gpuTimer`.
    #[wasm_bindgen(js_name = gpuFrameUs)]
    pub gpu_frame_us: u32,
    /// Адаптер даёт `timestamp-query`; без него `gpuFrameUs` всегда 0.
    #[wasm_bindgen(js_name = gpuTimer)]
    pub gpu_timer: bool,
    /// Полная загрузка мира, мс; 0, пока идёт. Считается от создания движка или `set_camera_state`.
    #[wasm_bindgen(js_name = loadMs)]
    pub load_ms: u32,
}

/// Блок из таблицы `BLOCKS` (`blocks()`), по порядку номеров.
#[wasm_bindgen]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockInfo {
    /// Имя из JS API (`"tall_grass"`): его принимают `select_block` и `block_icon`.
    #[wasm_bindgen(getter_with_clone)]
    pub name: String,
    /// Русская подпись для игрока.
    #[wasm_bindgen(getter_with_clone)]
    pub label: String,
    /// Игрок может поставить блок (`select_block` его примет): не воздух и не вода.
    pub placeable: bool,
}

/// Список блоков в порядке номеров (номер блока = индекс).
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

/// Настройки, которыми управляет движок (`Engine::apply_settings`). Движок зажимает значения
/// в пределы [`SettingsLimits`] сам. Клиентские настройки (`showDebug`) сюда не входят.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EngineSettings {
    /// Дальность прорисовки, колонок.
    #[wasm_bindgen(js_name = renderDistance)]
    pub render_distance: i32,
    /// Затенение углов (AO).
    #[wasm_bindgen(js_name = ambientOcclusion)]
    pub ambient_occlusion: bool,
    /// Множитель чувствительности мыши к базовым 0.002 рад/пиксель.
    #[wasm_bindgen(js_name = mouseSensitivity)]
    pub mouse_sensitivity: f32,
    /// Вертикальный угол обзора, градусы.
    pub fov: f32,
}

#[wasm_bindgen]
impl EngineSettings {
    #[wasm_bindgen(constructor)]
    pub fn new(
        render_distance: i32,
        ambient_occlusion: bool,
        mouse_sensitivity: f32,
        fov: f32,
    ) -> EngineSettings {
        EngineSettings {
            render_distance,
            ambient_occlusion,
            mouse_sensitivity,
            fov,
        }
    }
}

/// Значения настроек по умолчанию.
#[wasm_bindgen]
pub fn default_settings() -> EngineSettings {
    EngineSettings {
        render_distance: DEFAULT_RADIUS,
        ambient_occlusion: MeshOptions::default().ambient_occlusion,
        mouse_sensitivity: DEFAULT_SENSITIVITY,
        fov: DEFAULT_FOV,
    }
}

/// Пределы настроек: фронт строит по ним ползунки и зажимает прочитанные значения.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SettingsLimits {
    /// Дальность прорисовки, колонок.
    #[wasm_bindgen(js_name = renderDistanceMin)]
    pub render_distance_min: i32,
    #[wasm_bindgen(js_name = renderDistanceMax)]
    pub render_distance_max: i32,
    /// Множитель чувствительности мыши.
    #[wasm_bindgen(js_name = mouseSensitivityMin)]
    pub mouse_sensitivity_min: f32,
    #[wasm_bindgen(js_name = mouseSensitivityMax)]
    pub mouse_sensitivity_max: f32,
    /// Вертикальный угол обзора, градусы.
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
