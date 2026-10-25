/// Вершина: позиция (vec3), UV (vec2), яркость стороны (f32), слой текстуры (u32), 28 байт.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 3],
    /// В размерах блока: 0..1 на грань, v = 0 — верх тайла.
    pub uv: [f32; 2],
    /// Множитель яркости грани по её стороне.
    pub shade: f32,
    pub layer: u32,
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32, 3 => Uint32];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub fn vertex_bytes(vertices: &[Vertex]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(size_of_val(vertices));
    for v in vertices {
        for f in v.position.into_iter().chain(v.uv).chain([v.shade]) {
            bytes.extend(f.to_le_bytes());
        }
        bytes.extend(v.layer.to_le_bytes());
    }
    bytes
}

pub fn index_bytes(indices: &[u16]) -> Vec<u8> {
    indices.iter().flat_map(|i| i.to_le_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_bytes_match_layout() {
        let v = Vertex {
            position: [1.0, 2.0, 3.0],
            uv: [0.0, 1.0],
            shade: 0.5,
            layer: 7,
        };
        let bytes = vertex_bytes(&[v, v]);
        assert_eq!(size_of::<Vertex>(), 28);
        assert_eq!(bytes.len(), 2 * 28);
        assert_eq!(&bytes[20..24], &0.5f32.to_le_bytes());
        assert_eq!(&bytes[24..28], &7u32.to_le_bytes());
    }
}
