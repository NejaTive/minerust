use glam::Vec3;
use mesh::mesh::{QUAD_BYTES, quad_bytes};
use mesh::mesher::{MeshData, SectionMeshes};
use world::column::COLUMN_SECTIONS;
use world::section::SECTION_SIZE;

use crate::arena::{QuadArena, WORD};
use crate::quad_indices::{INDICES_PER_QUAD, QuadIndices};
use crate::slots::SlotBuffer;

/// Общие буферы мешей: записи квадов всех секций, слоты мешей и индексы квадов.
pub(crate) struct SceneBuffers {
    pub(crate) arena: QuadArena,
    pub(crate) slots: SlotBuffer,
    pub(crate) indices: QuadIndices,
}

impl SceneBuffers {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        SceneBuffers {
            arena: QuadArena::new(device),
            slots: SlotBuffer::new(device),
            indices: QuadIndices::new(device),
        }
    }

    /// Размер буферов на GPU, байт.
    pub(crate) fn bytes(&self) -> u64 {
        self.arena.capacity() + self.slots.bytes() + self.indices.bytes()
    }

    /// Занято мешами и слотами, байт.
    pub(crate) fn used_bytes(&self) -> u64 {
        // Индексный шаблон занят целиком: свободных мест в нём нет.
        self.arena.used() + self.slots.used_bytes() + self.indices.bytes()
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

    /// Пайплайн, арена (bind group), буфер слотов и индексы квадов уже выставлены. Индексы
    /// общие, `base_vertex` нулевой: квады меша нумеруются с нуля; слот — `first_instance`,
    /// по нему вершинный шейдер берёт угол секции и начало записей меша.
    pub(crate) fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.draw_indexed(
            0..INDICES_PER_QUAD * self.quad_count,
            0,
            self.slot..self.slot + 1,
        );
    }
}

/// Меши одной секции на GPU по проходам; хотя бы один не `None`.
pub(crate) struct SectionMesh {
    pub(crate) solid: Option<GpuMesh>,
    pub(crate) cutout: Option<GpuMesh>,
    pub(crate) water: Option<GpuMesh>,
    /// Коробка секции целиком в мировых координатах — для отсечения по пирамиде.
    pub(crate) min: Vec3,
    pub(crate) max: Vec3,
}

impl SectionMesh {
    /// Возвращает диапазоны и слоты секции буферам.
    pub(crate) fn release(self, buffers: &mut SceneBuffers) {
        for mesh in [self.solid, self.cutout, self.water].into_iter().flatten() {
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
    let meshes = [&data.solid, &data.cutout, &data.water];
    // Индексы сначала: они не привязаны к мешу, и при ошибке откатывать нечего.
    let most = meshes.iter().map(|m| m.quads.len()).max().unwrap_or(0);
    buffers.indices.ensure(
        device,
        u32::try_from(most).map_err(|_| "mesh has too many quads")?,
    )?;
    let mut uploaded: Vec<Option<GpuMesh>> = Vec::with_capacity(meshes.len());
    for data in meshes {
        match GpuMesh::upload(device, queue, buffers, data, min) {
            Ok(mesh) => uploaded.push(mesh),
            Err(e) => {
                for mesh in uploaded.into_iter().flatten() {
                    mesh.free(buffers);
                }
                return Err(e);
            }
        }
    }
    let [solid, cutout, water]: [Option<GpuMesh>; 3] =
        uploaded.try_into().unwrap_or_else(|_| unreachable!());
    if solid.is_none() && cutout.is_none() && water.is_none() {
        return Ok(None);
    }
    Ok(Some(SectionMesh {
        solid,
        cutout,
        water,
        min,
        max: min + SECTION_SIZE as f32,
    }))
}
