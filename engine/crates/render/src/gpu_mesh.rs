use glam::Vec3;
use mesh::mesh::{gpu_vertices, vertex_bytes};
use mesh::mesher::{MeshData, SectionMeshes};
use world::column::COLUMN_SECTIONS;
use world::section::SECTION_SIZE;

use crate::arena::{VERTEX_SIZE, VertexArena};
use crate::slots::OriginBuffer;

/// Общие буферы мешей: вершины всех секций и их углы.
pub(crate) struct SceneBuffers {
    pub(crate) arena: VertexArena,
    pub(crate) origins: OriginBuffer,
}

impl SceneBuffers {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        SceneBuffers {
            arena: VertexArena::new(device),
            origins: OriginBuffer::new(device),
        }
    }

    /// Размер буферов на GPU, байт.
    pub(crate) fn bytes(&self) -> u64 {
        self.arena.capacity() + self.origins.bytes()
    }

    /// Занято мешами и углами, байт.
    pub(crate) fn used_bytes(&self) -> u64 {
        self.arena.used() + self.origins.used_bytes()
    }
}

/// Диапазон вершин одного меша в арене; индексы общие (`Renderer::quad_index_buffer`).
pub(crate) struct GpuMesh {
    /// Смещение в арене, байт; кратно размеру вершины.
    offset: u64,
    bytes: u64,
    pub(crate) vertex_count: u32,
    pub(crate) index_count: u32,
}

impl GpuMesh {
    /// `Ok(None)` для пустого меша: диапазон нулевого размера не нужен.
    pub(crate) fn upload(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        arena: &mut VertexArena,
        data: &MeshData,
    ) -> Result<Option<GpuMesh>, String> {
        if data.vertices.is_empty() {
            return Ok(None);
        }
        let bytes = vertex_bytes(&gpu_vertices(&data.vertices, &data.indices));
        let offset = arena.write(device, queue, &bytes)?;
        Ok(Some(GpuMesh {
            offset,
            bytes: bytes.len() as u64,
            vertex_count: data.vertices.len() as u32,
            index_count: data.indices.len() as u32,
        }))
    }

    fn free(self, arena: &mut VertexArena) {
        arena.free(self.offset, self.bytes);
    }

    /// Пайплайн, bind group, индексы, арена и буфер углов уже выставлены; `slot` секции —
    /// `first_instance`, по нему вершинный шейдер берёт угол. `base_vertex` сдвигает
    /// локальные u16-индексы меша на его начало в арене.
    pub(crate) fn draw(&self, pass: &mut wgpu::RenderPass<'_>, slot: u32) {
        let base_vertex = (self.offset / VERTEX_SIZE) as i32;
        pass.draw_indexed(0..self.index_count, base_vertex, slot..slot + 1);
    }
}

/// Меши одной секции на GPU по проходам; хотя бы один не `None`.
pub(crate) struct SectionMesh {
    pub(crate) solid: Option<GpuMesh>,
    pub(crate) water: Option<GpuMesh>,
    /// Слот угла секции в [`OriginBuffer`]: один на оба прохода.
    pub(crate) slot: u32,
    /// Коробка секции целиком в мировых координатах — для отсечения по пирамиде.
    pub(crate) min: Vec3,
    pub(crate) max: Vec3,
}

impl SectionMesh {
    /// Возвращает диапазоны и слот секции буферам.
    pub(crate) fn release(self, buffers: &mut SceneBuffers) {
        for mesh in [self.solid, self.water].into_iter().flatten() {
            mesh.free(&mut buffers.arena);
        }
        buffers.origins.free(self.slot);
    }
}

/// Меши секций колонки снизу вверх; `None` — секция без видимых граней. Номер
/// секции нужен, чтобы после правки блока перестроить одну её, а не всю колонку.
pub(crate) type ColumnMeshes = [Option<SectionMesh>; COLUMN_SECTIONS as usize];

/// `min` — нижний угол секции в мире. `Ok(None)` для секции без граней. При ошибке
/// (буфер на пределе) всё уже выделенное возвращается.
pub(crate) fn upload_section(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffers: &mut SceneBuffers,
    data: &SectionMeshes,
    min: Vec3,
) -> Result<Option<SectionMesh>, String> {
    let solid = GpuMesh::upload(device, queue, &mut buffers.arena, &data.solid)?;
    let water = match GpuMesh::upload(device, queue, &mut buffers.arena, &data.water) {
        Ok(water) => water,
        Err(e) => {
            if let Some(solid) = solid {
                solid.free(&mut buffers.arena);
            }
            return Err(e);
        }
    };
    if solid.is_none() && water.is_none() {
        return Ok(None);
    }
    let slot = match buffers.origins.write(device, queue, min.to_array()) {
        Ok(slot) => slot,
        Err(e) => {
            for mesh in [solid, water].into_iter().flatten() {
                mesh.free(&mut buffers.arena);
            }
            return Err(e);
        }
    };
    Ok(Some(SectionMesh {
        solid,
        water,
        slot,
        min,
        max: min + SECTION_SIZE as f32,
    }))
}
