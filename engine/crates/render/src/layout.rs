use mesh::mesh::SLOT_BYTES;

/// Слот меша на один экземпляр: слот в общем буфере — `first_instance` отрисовки. Так
/// буфер ставится раз на проход, без push-констант (в WebGPU их нет) и bind group на
/// секцию. Вершинных буферов у блоков нет: квады шейдер читает из арены по `vertex_index`.
/// Атрибуты: 0 — мировой угол секции (`vec3<f32>`), 1 — начало записей квадов меша в
/// арене в словах `u32` (по смещению 12, там у `vec3<f32>` пустовали байты выравнивания).
const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Uint32];

pub fn slot_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: SLOT_BYTES as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &ATTRIBUTES,
    }
}
