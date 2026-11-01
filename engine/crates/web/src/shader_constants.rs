use game::daytime::DUSK_SKY;
use game::rain_cover::COVER_SIZE;
use render::shaders::{BLOCK_SHADER, CLOUDS_SHADER, RAIN_SHADER, SKY_SHADER};
use world::clouds::{CLOUD_CELL, CLOUD_HEIGHT, CLOUD_MAP, CLOUD_Y};
use world::light::MAX_LIGHT;
use world::light::level_brightness;

#[test]
fn sky_shader_constants_match_cpu() {
    // Закатный цвет в шейдере неба дублирует `DUSK_SKY`: тёплое пятно должно быть
    // того же оттенка, что и тонировка неба.
    let [r, g, b] = DUSK_SKY;
    let dusk = format!("const DUSK_COLOR: vec3<f32> = vec3<f32>({r}, {g}, {b});");
    assert!(SKY_SHADER.contains(&dusk));
    assert!(BLOCK_SHADER.contains(&dusk));
}

#[test]
fn shader_constants_match_cpu() {
    assert!(BLOCK_SHADER.contains("const LIGHT_BASE: f32 = 0.8;"));
    assert!(BLOCK_SHADER.contains("const MIN_BRIGHTNESS: f32 = 0.05;"));
    assert!(BLOCK_SHADER.contains("const MAX_LIGHT: f32 = 15.0;"));
    assert_eq!(MAX_LIGHT, 15);
    assert_eq!(level_brightness(0.0), 0.8f32.powi(15).max(0.05));
}

#[test]
fn cloud_shader_constants_match_cpu() {
    assert!(CLOUDS_SHADER.contains(&format!("const CLOUD_MAP: i32 = {CLOUD_MAP};")));
    assert!(CLOUDS_SHADER.contains(&format!("const CLOUD_CELL: f32 = {CLOUD_CELL}.0;")));
    assert!(CLOUDS_SHADER.contains(&format!("const CLOUD_Y: f32 = {CLOUD_Y}.0;")));
    assert!(CLOUDS_SHADER.contains(&format!("const CLOUD_HEIGHT: f32 = {CLOUD_HEIGHT}.0;")));
    assert!(CLOUDS_SHADER.contains("const GRID_RADIUS: i32 = 32;"));
    assert!(CLOUDS_SHADER.contains("const GRID_SIZE: u32 = 64u;"));
}

#[test]
fn rain_shader_grid_matches_cover_window() {
    assert!(RAIN_SHADER.contains(&format!("const GRID_SIZE: u32 = {COVER_SIZE}u;")));
    assert!(RAIN_SHADER.contains(&format!("const GRID_RADIUS: i32 = {};", COVER_SIZE / 2)));
}
