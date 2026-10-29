//! Задание и результат мешера для воркера. Основной поток заполняет кэши секций 18³
//! ([`Surroundings`]), воркер строит из них меши и ничего не знает про мир: на секцию
//! уходит ~11.4 КиБ, назад возвращаются только квады.
//!
//! Все числа — little endian.
//! - Задание: `id: u32`, `ambient_occlusion: u8`, `count: u8`, затем `count` секций:
//!   `sy: u8`, блоки кэша (`PADDED³` байт), свет кэша (`PADDED³` байт).
//! - Результат: `id: u32`, `count: u8`, затем `count` секций: `sy: u8` и три меша
//!   (`solid`, `cutout`, `water`), у каждого `quads: u32` и столько записей по 12 байт.
use world::block::Block;
use world::column::COLUMN_SECTIONS;
use world::light::LightMap;
use world::pos::{ColumnPos, SectionPos};
use world::world::World;

use crate::mesh::{QUAD_BYTES, Quad};
use crate::mesher::{
    Lights, MeshData, MeshOptions, Neighborhood, PADDED, SectionMeshes, Surroundings, mesh_cached,
    new_caches,
};

const CACHE_LEN: usize = (PADDED * PADDED * PADDED) as usize;
const HEADER_LEN: usize = 6;
const SECTION_JOB_LEN: usize = 1 + 2 * CACHE_LEN;

/// Результат задания: меши непустых секций, снизу вверх.
pub struct JobResult {
    pub id: u32,
    /// `(номер секции в колонке, меши)`. Секции, которых нет в задании (воздух), не вошли.
    pub sections: Vec<(u8, SectionMeshes)>,
}

impl JobResult {
    /// Меши всех секций колонки для `Renderer::upload_column`; пропущенные — пустые.
    pub fn into_column(self) -> Vec<SectionMeshes> {
        let mut column: Vec<SectionMeshes> = (0..COLUMN_SECTIONS)
            .map(|_| SectionMeshes::empty())
            .collect();
        for (sy, meshes) in self.sections {
            column[usize::from(sy)] = meshes;
        }
        column
    }

    /// Меш единственной секции задания; пустой, если секция была воздухом.
    pub fn into_section(mut self) -> SectionMeshes {
        self.sections
            .pop()
            .map_or_else(SectionMeshes::empty, |(_, meshes)| meshes)
    }
}

fn start_job(id: u32, options: MeshOptions) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(HEADER_LEN + 8 * SECTION_JOB_LEN);
    bytes.extend_from_slice(&id.to_le_bytes());
    bytes.push(u8::from(options.ambient_occlusion));
    bytes.push(0);
    bytes
}

fn push_section(
    bytes: &mut Vec<u8>,
    sy: i32,
    around: &Surroundings,
    blocks: &mut Neighborhood,
    lights: &mut Lights,
) {
    around.fill(sy, blocks, lights);
    bytes.push(sy as u8);
    // `Block` — `repr(u8)`, поэтому побайтовая запись совпадает с номерами блоков.
    bytes.extend(blocks.0.iter().map(|&b| b as u8));
    bytes.extend_from_slice(&lights.0);
    bytes[HEADER_LEN - 1] += 1;
}

/// Задание на все непустые секции колонки `key`; `None`, если колонка не загружена.
/// Колонка без непустых секций даёт задание с `count = 0`.
pub fn encode_column_job(
    world: &World,
    light: &LightMap,
    id: u32,
    key: ColumnPos,
    options: MeshOptions,
) -> Option<Vec<u8>> {
    let around = Surroundings::new(world, light, key)?;
    let column = around.columns[4]?;
    let mut bytes = start_job(id, options);
    let (mut blocks, mut lights) = new_caches();
    for sy in 0..COLUMN_SECTIONS {
        if !column.section(sy).is_air() {
            push_section(&mut bytes, sy, &around, &mut blocks, &mut lights);
        }
    }
    Some(bytes)
}

/// Задание на одну секцию; воздух даёт задание без секций. `None`, если колонка не
/// загружена или номер секции вне колонки.
pub fn encode_section_job(
    world: &World,
    light: &LightMap,
    id: u32,
    key: SectionPos,
    options: MeshOptions,
) -> Option<Vec<u8>> {
    if !(0..COLUMN_SECTIONS).contains(&key.y()) {
        return None;
    }
    let around = Surroundings::new(world, light, key.column())?;
    let mut bytes = start_job(id, options);
    if !around.columns[4]?.section(key.y()).is_air() {
        let (mut blocks, mut lights) = new_caches();
        push_section(&mut bytes, key.y(), &around, &mut blocks, &mut lights);
    }
    Some(bytes)
}

/// Строит меши по заданию и пакует результат. Паникует на заданиях не из
/// `encode_*_job`: границу пересекают только свои байты.
pub fn run_job(job: &[u8]) -> Vec<u8> {
    let id = u32::from_le_bytes(job[..4].try_into().expect("job header"));
    let options = MeshOptions {
        ambient_occlusion: job[4] != 0,
    };
    let count = usize::from(job[5]);
    assert_eq!(
        job.len(),
        HEADER_LEN + count * SECTION_JOB_LEN,
        "job length"
    );
    let (mut blocks, mut lights) = new_caches();
    let mut out = Vec::new();
    out.extend_from_slice(&id.to_le_bytes());
    out.push(count as u8);
    for chunk in job[HEADER_LEN..].as_chunks::<SECTION_JOB_LEN>().0 {
        for (dst, &src) in blocks.0.iter_mut().zip(&chunk[1..1 + CACHE_LEN]) {
            *dst = Block::try_from(src).expect("job block id");
        }
        lights.0.copy_from_slice(&chunk[1 + CACHE_LEN..]);
        let meshes = mesh_cached(&blocks, &lights, options);
        out.push(chunk[0]);
        for mesh in [&meshes.solid, &meshes.cutout, &meshes.water] {
            out.extend_from_slice(&(mesh.quads.len() as u32).to_le_bytes());
            for quad in &mesh.quads {
                out.extend(quad.words().into_iter().flat_map(u32::to_le_bytes));
            }
        }
    }
    out
}

/// Разбирает результат [`run_job`]. Паникует на чужих байтах, как и `run_job`.
pub fn decode_result(bytes: &[u8]) -> JobResult {
    let id = u32::from_le_bytes(bytes[..4].try_into().expect("result header"));
    let count = usize::from(bytes[4]);
    let mut at = 5;
    let mut sections = Vec::with_capacity(count);
    for _ in 0..count {
        let sy = bytes[at];
        at += 1;
        let mut read_mesh = || {
            let len =
                u32::from_le_bytes(bytes[at..at + 4].try_into().expect("quad count")) as usize;
            at += 4;
            let (words, _) = bytes[at..at + len * QUAD_BYTES].as_chunks::<4>();
            let words: Vec<u32> = words.iter().map(|w| u32::from_le_bytes(*w)).collect();
            let (triples, _) = words.as_chunks::<3>();
            let quads = triples.iter().map(|&w| Quad::from_words(w)).collect();
            at += len * QUAD_BYTES;
            MeshData { quads }
        };
        let (solid, cutout, water) = (read_mesh(), read_mesh(), read_mesh());
        sections.push((
            sy,
            SectionMeshes {
                solid,
                cutout,
                water,
            },
        ));
    }
    assert_eq!(at, bytes.len(), "result length");
    JobResult { id, sections }
}

#[cfg(test)]
mod tests;
