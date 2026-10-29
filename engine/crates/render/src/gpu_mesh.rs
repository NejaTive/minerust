use glam::Vec3;
use mesh::mesh::{QUAD_BYTES, quad_bytes};
use mesh::mesher::{MeshData, SectionMeshes};
use world::column::COLUMN_SECTIONS;
use world::section::SECTION_SIZE;

use crate::arena::{QuadArena, WORD};
use crate::slots::SlotBuffer;

/// Общие буферы мешей: записи квадов всех секций и слоты мешей.
pub(crate) struct SceneBuffers {
    pub(crate) arena: QuadArena,
    pub(crate) slots: SlotBuffer,
}

impl SceneBuffers {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        SceneBuffers {
            arena: QuadArena::new(device),
            slots: SlotBuffer::new(device),
        }
    }

    /// Размер буферов на GPU, байт.
    pub(crate) fn bytes(&self) -> u64 {
        self.arena.capacity() + self.slots.bytes()
    }

    /// Занято мешами и слотами, байт.
    pub(crate) fn used_bytes(&self) -> u64 {
        self.arena.used() + self.slots.used_bytes()
    }
}

/// Записи квадов одного меша в арене и его слот.
pub(crate) struct GpuMesh {
    /// Смещение в арене, байт; кратно слову.
    offset: u64,
    bytes: u64,
    pub(crate) quad_count: u32,
    /// Слот в [`SlotBuffer`]: угол секции и начало записей; он же `first_instance`.
    slot: u32,
}

impl GpuMesh {
    /// `Ok(None)` для пустого меша: диапазон нулевого размера не нужен. `min` — нижний
    /// угол секции в мире.
    pub(crate) fn upload(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffers: &mut SceneBuffers,
        data: &MeshData,
        min: Vec3,
    ) -> Result<Option<GpuMesh>, String> {
        if data.quads.is_empty() {
            return Ok(None);
        }
        let bytes = quad_bytes(&data.quads);
        let offset = buffers.arena.write(device, queue, &bytes)?;
        let first_word = u32::try_from(offset / WORD).expect("arena is below 16 GiB");
        let slot = match buffers
            .slots
            .write(device, queue, min.to_array(), first_word)
        {
            Ok(slot) => slot,
            Err(e) => {
                buffers.arena.free(offset, bytes.len() as u64);
                return Err(e);
            }
        };
        debug_assert_eq!(bytes.len(), data.quads.len() * QUAD_BYTES);
        Ok(Some(GpuMesh {
            offset,
            bytes: bytes.len() as u64,
            quad_count: data.quads.len() as u32,
            slot,
        }))
    }

    fn free(self, buffers: &mut SceneBuffers) {
        buffers.arena.free(self.offset, self.bytes);
        buffers.slots.free(self.slot);
    }

    /// Пайплайн, арена (bind group) и буфер слотов уже выставлены. Шесть вершин на квад
    /// без индексов; слот — `first_instance`, по нему вершинный шейдер берёт угол секции
    /// и начало записей меша.
    pub(crate) fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.draw(0..6 * self.quad_count, self.slot..self.slot + 1);
    }
}

/// Меши одной секции на GPU по проходам; хотя бы один не `None`.
pub(crate) struct SectionMesh {
    pub(crate) solid: Option<GpuMesh>,
    pub(crate) water: Option<GpuMesh>,
    /// Коробка секции целиком в мировых координатах — для отсечения по пирамиде.
    pub(crate) min: Vec3,
    pub(crate) max: Vec3,
}

impl SectionMesh {
    /// Возвращает диапазоны и слоты секции буферам.
    pub(crate) fn release(self, buffers: &mut SceneBuffers) {
        for mesh in [self.solid, self.water].into_iter().flatten() {
            mesh.free(buffers);
        }
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
    let solid = GpuMesh::upload(device, queue, buffers, &data.solid, min)?;
    let water = match GpuMesh::upload(device, queue, buffers, &data.water, min) {
        Ok(water) => water,
        Err(e) => {
            if let Some(solid) = solid {
                solid.free(buffers);
            }
            return Err(e);
        }
    };
    if solid.is_none() && water.is_none() {
        return Ok(None);
    }
    Ok(Some(SectionMesh {
        solid,
        water,
        min,
        max: min + SECTION_SIZE as f32,
    }))
}
