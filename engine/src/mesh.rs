/// Вершина: позиция (vec3) + цвет (vec3), 24 байта.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

const GRASS: [f32; 3] = [0.30, 0.60, 0.20];
const DIRT: [f32; 3] = [0.45, 0.30, 0.15];
// Боковые грани — землистые оттенки разной яркости, чтобы грани различались.
const SIDE_X: [f32; 3] = [0.55, 0.38, 0.20];
const SIDE_Z: [f32; 3] = [0.40, 0.27, 0.13];

/// Куб размером 1 с центром в (0, 0, 0): 24 вершины (по 4 на грань) и 36 индексов.
/// Вершины каждой грани идут против часовой стрелки, если смотреть снаружи.
pub fn cube() -> (Vec<Vertex>, Vec<u16>) {
    let h = 0.5;
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        // +Y (верх)
        (GRASS, [[-h, h, h], [h, h, h], [h, h, -h], [-h, h, -h]]),
        // -Y (низ)
        (DIRT, [[-h, -h, -h], [h, -h, -h], [h, -h, h], [-h, -h, h]]),
        // +X
        (SIDE_X, [[h, -h, h], [h, -h, -h], [h, h, -h], [h, h, h]]),
        // -X
        (
            SIDE_X.map(|c| c * 0.8),
            [[-h, -h, -h], [-h, -h, h], [-h, h, h], [-h, h, -h]],
        ),
        // +Z
        (SIDE_Z, [[-h, -h, h], [h, -h, h], [h, h, h], [-h, h, h]]),
        // -Z
        (
            SIDE_Z.map(|c| c * 0.8),
            [[h, -h, -h], [-h, -h, -h], [-h, h, -h], [h, h, -h]],
        ),
    ];

    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (color, corners) in faces {
        let base = vertices.len() as u16;
        vertices.extend(corners.map(|position| Vertex { position, color }));
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (vertices, indices)
}

pub fn vertex_bytes(vertices: &[Vertex]) -> Vec<u8> {
    vertices
        .iter()
        .flat_map(|v| v.position.into_iter().chain(v.color))
        .flat_map(f32::to_le_bytes)
        .collect()
}

pub fn index_bytes(indices: &[u16]) -> Vec<u8> {
    indices.iter().flat_map(|i| i.to_le_bytes()).collect()
}
