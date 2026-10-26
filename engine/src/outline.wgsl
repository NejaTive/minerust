// Начало `Globals` из shader.wgsl: рамке нужна только матрица. Буфер тот же,
// и в шейдере достаточно префикса структуры.
struct Globals {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> globals: Globals;

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return globals.view_proj * vec4<f32>(position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // Почти чёрный: видно и на траве, и на камне, и на фоне неба.
    return vec4<f32>(0.02, 0.02, 0.02, 1.0);
}
