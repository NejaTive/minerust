// Модули без GPU и браузера публичны: так они собираются и тестируются на хосте
// (`cargo test`) без предупреждений о мёртвом коде. Всё, что держится на canvas
// (`wgpu::SurfaceTarget::Canvas` есть только на wasm32), закрыто cfg ниже.
pub mod block;
pub mod column;
pub mod loader;
pub mod mesh;
pub mod mesher;
pub mod section;
pub mod terrain;
pub mod texture;
pub mod world;

#[cfg(target_arch = "wasm32")]
mod camera;
#[cfg(target_arch = "wasm32")]
mod engine;

#[cfg(target_arch = "wasm32")]
pub use engine::Engine;
