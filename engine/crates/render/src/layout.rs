use mesh::mesh::{ORIGIN_BYTES, Vertex};

/// Вершина секции — два `u32` ([`mesh::mesh::Vertex`]).
pub fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Uint32x2],
    }
}

/// Мировой угол секции (3 × f32, 16 байт с выравниванием) на один экземпляр: слот секции
/// в общем буфере — `first_instance` отрисовки. Так буфер ставится раз на проход, без
/// push-констант и bind group на секцию.
pub fn origin_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: ORIGIN_BYTES as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![1 => Float32x3],
    }
}
