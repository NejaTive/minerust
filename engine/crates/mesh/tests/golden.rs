//! Golden-хеши генерации и мешей: страхуют рефакторинг от тихой смены поведения.
//! Константы меняются только при намеренной смене генерации, света или формата меша.
use glam::IVec3;

use mesh::mesh::quad_bytes;
use mesh::mesher::{MeshOptions, mesh_world_column};
use world::column::WORLD_HEIGHT;
use world::light::LightMap;
use world::pos::ColumnPos;
use world::terrain::Terrain;
use world::world::World;

const SEED: i32 = 1337;
/// Равнина, лес у воды, пустыня, пещера.
const KEYS: [(i32, i32); 4] = [(-4, -11), (6, -12), (0, -12), (0, -7)];

/// FNV-1a, 64 бита.
fn fnv(hash: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *hash ^= u64::from(b);
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

const FNV_START: u64 = 0xcbf2_9ce4_8422_2325;

#[test]
fn generated_blocks_match_golden() {
    let terrain = Terrain::new(SEED);
    let hashes: Vec<u64> = KEYS
        .iter()
        .map(|&(x, z)| {
            let column = terrain.generate(ColumnPos::new(x, z));
            let mut hash = FNV_START;
            for y in 0..WORLD_HEIGHT {
                for lz in 0..16 {
                    for lx in 0..16 {
                        fnv(&mut hash, &[column.get(IVec3::new(lx, y, lz)) as u8]);
                    }
                }
            }
            hash
        })
        .collect();
    assert_eq!(hashes, GOLDEN_BLOCKS, "blocks: {hashes:#x?}");
}

#[test]
fn meshes_match_golden() {
    let terrain = Terrain::new(SEED);
    let mut world = World::new();
    for &(kx, kz) in &KEYS {
        for z in kz - 1..=kz + 1 {
            for x in kx - 1..=kx + 1 {
                let key = ColumnPos::new(x, z);
                if world.column(key).is_none() {
                    world.insert(key, terrain.generate(key));
                }
            }
        }
    }
    let mut light = LightMap::new();
    let mut keys: Vec<ColumnPos> = world.keys().collect();
    keys.sort();
    for key in keys {
        light.add_column(&world, key);
    }
    let hashes: Vec<u64> = KEYS
        .iter()
        .map(|&(x, z)| {
            let sections =
                mesh_world_column(&world, &light, ColumnPos::new(x, z), MeshOptions::default())
                    .expect("column is loaded");
            let mut hash = FNV_START;
            for section in &sections {
                for data in [&section.solid, &section.cutout, &section.water] {
                    fnv(&mut hash, &quad_bytes(&data.quads));
                }
            }
            hash
        })
        .collect();
    assert_eq!(hashes, GOLDEN_MESHES, "meshes: {hashes:#x?}");
}

const GOLDEN_BLOCKS: [u64; 4] = [
    0xab405e38f0b2b8e5,
    0x6bc62c67f44d3de8,
    0xe69a701bfc6595d0,
    0x4fe7fff820d365db,
];
/// Хеши записей квадов (`quad_bytes`) по слоям solid, cutout, water. В 20c листва, кресты и
/// факел ушли из solid в отдельный cutout: у колонок без них (равнина, пустыня) хеш прежний.
/// Формат поменялся в 20b (вершины → записи по 12 байт);
/// эквивалентность старого и нового меша один в один до смены была доказана тестом,
/// который развёртывал записи через шаблоны и сравнивал с прежними вершинами.
const GOLDEN_MESHES: [u64; 4] = [
    0xf8168e1e2bc0646c,
    0x6471aed5fe73aafe,
    0xbb91b5e8a5e32b86,
    0x20f612c7d843cfd2,
];
