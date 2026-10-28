use glam::Vec3;
use mesh::mesh::{gpu_vertices, origin_bytes, vertex_bytes};
use mesh::mesher::{MeshData, SectionMeshes};
use wgpu::util::DeviceExt;
use world::column::COLUMN_SECTIONS;
use world::section::SECTION_SIZE;

/// GPU-буфер вершин одного меша; индексы общие (`Renderer::quad_index_buffer`).
pub(crate) struct GpuMesh {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) vertex_count: u32,
    pub(crate) index_count: u32,
}

impl GpuMesh {
    /// `None` для пустого меша: буфер нулевого размера не нужен.
    pub(crate) fn upload(device: &wgpu::Device, data: &MeshData) -> Option<GpuMesh> {
        if data.vertices.is_empty() {
            return None;
        }
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("section vertices"),
            contents: &vertex_bytes(&gpu_vertices(&data.vertices, &data.indices)),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Some(GpuMesh {
            vertex_buffer,
            vertex_count: data.vertices.len() as u32,
            index_count: data.indices.len() as u32,
        })
    }

    /// Размер буфера вершин на GPU, байт.
    pub(crate) fn bytes(&self) -> u64 {
        self.vertex_buffer.size()
    }

    /// Пайплайн, bind group и индексы уже выставлены; `origin` — буфер угла секции ([`mesh::origin_layout`]).
    pub(crate) fn draw(&self, pass: &mut wgpu::RenderPass<'_>, origin: &wgpu::Buffer) {
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, origin.slice(..));
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

/// Меши одной секции на GPU по проходам; хотя бы один не `None`.
pub(crate) struct SectionMesh {
    pub(crate) solid: Option<GpuMesh>,
    pub(crate) water: Option<GpuMesh>,
    /// Мировой угол секции для вершинного шейдера: один буфер на оба прохода.
    pub(crate) origin: wgpu::Buffer,
    /// Коробка секции целиком в мировых координатах — для отсечения по пирамиде.
    pub(crate) min: Vec3,
    pub(crate) max: Vec3,
}

impl SectionMesh {
    /// Размер буферов секции на GPU, байт.
    pub(crate) fn bytes(&self) -> u64 {
        [&self.solid, &self.water]
            .into_iter()
            .flatten()
            .map(GpuMesh::bytes)
            .sum::<u64>()
            + self.origin.size()
    }
}

/// Меши секций колонки снизу вверх; `None` — секция без видимых граней. Номер
/// секции нужен, чтобы после правки блока перестроить одну её, а не всю колонку.
pub(crate) type ColumnMeshes = [Option<SectionMesh>; COLUMN_SECTIONS as usize];

/// `min` — нижний угол секции в мире. `None` для секции без граней.
pub(crate) fn upload_section(
    device: &wgpu::Device,
    data: &SectionMeshes,
    min: Vec3,
) -> Option<SectionMesh> {
    let solid = GpuMesh::upload(device, &data.solid);
    let water = GpuMesh::upload(device, &data.water);
    if solid.is_none() && water.is_none() {
        return None;
    }
    let origin = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("section origin"),
        contents: &origin_bytes(min.to_array()),
        usage: wgpu::BufferUsages::VERTEX,
    });
    Some(SectionMesh {
        solid,
        water,
        origin,
        min,
        max: min + SECTION_SIZE as f32,
    })
}
