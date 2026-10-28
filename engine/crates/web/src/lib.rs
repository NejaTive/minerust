// Весь код крейта — склейка под браузер: surface из canvas (`wgpu::SurfaceTarget::Canvas`)
// есть только на wasm32. На хосте собираются лишь тесты констант шейдеров.
pub mod api;
#[cfg(target_arch = "wasm32")]
mod engine;
#[cfg(target_arch = "wasm32")]
mod frame;
#[cfg(target_arch = "wasm32")]
mod profiler;
#[cfg(target_arch = "wasm32")]
mod streamer;

#[cfg(target_arch = "wasm32")]
pub use engine::{Engine, block_icon};

#[cfg(test)]
mod shader_constants;
