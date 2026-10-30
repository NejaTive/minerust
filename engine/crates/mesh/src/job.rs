use world::block::Block;
use world::column::COLUMN_SECTIONS;
use world::light::LightMap;
use world::pos::{ColumnPos, SectionPos};
use world::world::World;

use crate::mesh::{QUAD_BYTES, Quad};
use crate::mesher::{
    Datas, Lights, MeshData, MeshOptions, Neighborhood, PADDED, SectionMeshes, Surroundings,
    mesh_cached, new_caches,
};

const CACHE_LEN: usize = (PADDED * PADDED * PADDED) as usize;
const HEADER_LEN: usize = 6;
const SECTION_JOB_LEN: usize = 2 + 2 * CACHE_LEN;

pub struct JobResult {
    pub id: u32,
    pub sections: Vec<(u8, SectionMeshes)>,
}

impl JobResult {
    pub fn into_column(self) -> Vec<SectionMeshes> {
        let mut column: Vec<SectionMeshes> = (0..COLUMN_SECTIONS)
            .map(|_| SectionMeshes::empty())
            .collect();
        for (sy, meshes) in self.sections {
            column[usize::from(sy)] = meshes;
        }
        column
    }

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
    datas: &mut Datas,
) {
    let has_data = around.fill(sy, blocks, lights, datas);
    bytes.push(sy as u8);
    bytes.push(u8::from(has_data));
    // `Block` — `repr(u8)`, поэтому побайтовая запись совпадает с номерами блоков.
    bytes.extend(blocks.0.iter().map(|&b| b as u8));
    bytes.extend_from_slice(&lights.0);
    if has_data {
        bytes.extend_from_slice(&datas.0);
    }
    bytes[HEADER_LEN - 1] += 1;
}

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
    let (mut blocks, mut lights, mut datas) = new_caches();
    for sy in 0..COLUMN_SECTIONS {
        if !column.section(sy).is_air() {
            push_section(
                &mut bytes,
                sy,
                &around,
                &mut blocks,
                &mut lights,
                &mut datas,
            );
        }
    }
    Some(bytes)
}

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
        let (mut blocks, mut lights, mut datas) = new_caches();
        push_section(
            &mut bytes,
            key.y(),
            &around,
            &mut blocks,
            &mut lights,
            &mut datas,
        );
    }
    Some(bytes)
}

pub fn run_job(job: &[u8]) -> Vec<u8> {
    let id = u32::from_le_bytes(job[..4].try_into().expect("job header"));
    let options = MeshOptions {
        ambient_occlusion: job[4] != 0,
    };
    let count = usize::from(job[5]);
    let (mut blocks, mut lights, mut datas) = new_caches();
    let mut out = Vec::new();
    out.extend_from_slice(&id.to_le_bytes());
    out.push(count as u8);
    let mut at = HEADER_LEN;
    for _ in 0..count {
        let (sy, has_data) = (job[at], job[at + 1] != 0);
        let body = &job[at + 2..];
        for (dst, &src) in blocks.0.iter_mut().zip(&body[..CACHE_LEN]) {
            *dst = Block::try_from(src).expect("job block id");
        }
        lights.0.copy_from_slice(&body[CACHE_LEN..2 * CACHE_LEN]);
        at += SECTION_JOB_LEN;
        if has_data {
            datas.0.copy_from_slice(&job[at..at + CACHE_LEN]);
            at += CACHE_LEN;
        } else {
            datas.0.fill(0);
        }
        let meshes = mesh_cached(&blocks, &lights, &datas, options);
        out.push(sy);
        for mesh in [&meshes.solid, &meshes.cutout, &meshes.water] {
            out.extend_from_slice(&(mesh.quads.len() as u32).to_le_bytes());
            for quad in &mesh.quads {
                out.extend(quad.words().into_iter().flat_map(u32::to_le_bytes));
            }
        }
    }
    assert_eq!(at, job.len(), "job length");
    out
}

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
