//! WGSL-шейдеры как строки: тесты других крейтов сверяют их константы с CPU-кодом.

pub const BLOCK_SHADER: &str = include_str!("shader.wgsl");
pub const SKY_SHADER: &str = include_str!("sky.wgsl");
pub const CLOUDS_SHADER: &str = include_str!("clouds.wgsl");
pub const OUTLINE_SHADER: &str = include_str!("outline.wgsl");

pub fn module(device: &wgpu::Device, label: &str, source: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    })
}

#[cfg(test)]
mod tests;
