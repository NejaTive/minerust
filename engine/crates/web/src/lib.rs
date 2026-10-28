#[cfg(target_arch = "wasm32")]
mod engine;

#[cfg(target_arch = "wasm32")]
pub use engine::Engine;

#[cfg(test)]
mod shader_constants;
