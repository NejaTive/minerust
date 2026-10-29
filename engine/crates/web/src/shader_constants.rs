use game::daytime::DUSK_SKY;
use render::shaders::{BLOCK_SHADER, SKY_SHADER};
use world::light::MAX_LIGHT;
use world::light::level_brightness;

#[test]
fn sky_shader_constants_match_cpu() {
    // Закатный цвет в шейдере неба дублирует `DUSK_SKY`: тёплое пятно должно быть
    // того же оттенка, что и тонировка неба.
    let [r, g, b] = DUSK_SKY;
    assert!(SKY_SHADER.contains(&format!(
        "const DUSK_COLOR: vec3<f32> = vec3<f32>({r}, {g}, {b});"
    )));
}

#[test]
fn shader_constants_match_cpu() {
    assert!(BLOCK_SHADER.contains("const LIGHT_BASE: f32 = 0.8;"));
    assert!(BLOCK_SHADER.contains("const MIN_BRIGHTNESS: f32 = 0.05;"));
    assert!(BLOCK_SHADER.contains("const MAX_LIGHT: f32 = 15.0;"));
    assert_eq!(MAX_LIGHT, 15);
    assert_eq!(level_brightness(0.0), 0.8f32.powi(15).max(0.05));
}
