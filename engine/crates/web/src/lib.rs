// Весь код крейта — склейка под браузер: surface из canvas (`wgpu::SurfaceTarget::Canvas`)
// есть только на wasm32. На хосте собираются лишь тесты констант шейдеров.
pub mod api;
#[cfg(target_arch = "wasm32")]
mod engine;
#[cfg(target_arch = "wasm32")]
mod frame;
// Чистый учёт заданий воркеров: тестируется на хосте, в сборке для хоста не используется.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
mod jobs;
#[cfg(target_arch = "wasm32")]
mod profiler;
#[cfg(target_arch = "wasm32")]
mod streamer;

#[cfg(target_arch = "wasm32")]
pub use engine::{Engine, block_icon};

#[cfg(test)]
mod shader_constants;
