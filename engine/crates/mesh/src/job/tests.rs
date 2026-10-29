use super::*;
use crate::mesher::{mesh_world_column, mesh_world_section};
use world::block::{
    ATTACH_NEG_X, ATTACH_NEG_Z, ATTACH_POS_X, ATTACH_POS_Z, AXIS_X, AXIS_Y, AXIS_Z, Block,
};
use world::terrain::Terrain;

const SEED: i32 = 1337;
/// Равнина, лес у воды, пустыня, пещера (как в golden-тестах).
const KEYS: [(i32, i32); 4] = [(-4, -11), (6, -12), (0, -12), (0, -7)];

fn world_around(keys: &[(i32, i32)]) -> (World, LightMap) {
    let terrain = Terrain::new(SEED);
    let mut world = World::new();
    for &(kx, kz) in keys {
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
    for key in world.keys().collect::<Vec<_>>() {
        light.add_column(&world, key);
    }
    (world, light)
}

fn same(a: &SectionMeshes, b: &SectionMeshes) -> bool {
    a.solid.quads == b.solid.quads
        && a.cutout.quads == b.cutout.quads
        && a.water.quads == b.water.quads
}

#[test]
fn column_job_matches_direct_meshing() {
    let (world, light) = world_around(&KEYS);
    for ao in [true, false] {
        let options = MeshOptions {
            ambient_occlusion: ao,
        };
        for &(x, z) in &KEYS {
            let key = ColumnPos::new(x, z);
            let job = encode_column_job(&world, &light, 7, key, options).unwrap();
            let result = decode_result(&run_job(&job));
            assert_eq!(result.id, 7);
            let got = result.into_column();
            let want = mesh_world_column(&world, &light, key, options).unwrap();
            assert_eq!(got.len(), want.len());
            assert!(
                got.iter().zip(&want).all(|(a, b)| same(a, b)),
                "{key:?} ao={ao}"
            );
        }
    }
}

#[test]
fn section_job_matches_direct_meshing() {
    let (mut world, mut light) = world_around(&KEYS);
    let key = ColumnPos::new(0, -7);
    // Листва, крест, факел и вода рядом: все слои в одной секции.
    let base = key.origin() + glam::IVec3::new(4, 70, 4);
    world.set_block(base, Block::Leaves);
    world.set_block(base + glam::IVec3::X * 2, Block::Torch);
    light.add_column(&world, key);
    let options = MeshOptions::default();
    let mut checked = 0;
    for sy in -1..=COLUMN_SECTIONS {
        let section = SectionPos::new(key.x(), sy, key.z());
        let job = encode_section_job(&world, &light, 3, section, options);
        let want = mesh_world_section(&world, &light, section, options);
        assert_eq!(job.is_some(), want.is_some(), "section {sy}");
        if let (Some(job), Some(want)) = (job, want) {
            assert!(
                same(&decode_result(&run_job(&job)).into_section(), &want),
                "section {sy}"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, COLUMN_SECTIONS as usize);
}

#[test]
fn unloaded_column_has_no_job() {
    let world = World::new();
    let light = LightMap::new();
    let key = ColumnPos::new(0, 0);
    assert!(encode_column_job(&world, &light, 0, key, MeshOptions::default()).is_none());
}

/// Мир с ориентированными блоками в двух секциях колонки: лежачие брёвна, настенные
/// факелы и блок без данных рядом.
fn oriented_world() -> (World, LightMap, ColumnPos) {
    let (mut world, mut light) = world_around(&KEYS);
    let key = ColumnPos::new(0, -7);
    let base = key.origin() + glam::IVec3::new(4, 64, 4);
    // Секция, на стыке которой лежит бревно: данные сами по себе в секции 4 (y 64..80).
    for (i, data) in [AXIS_X, AXIS_Z, AXIS_Y].into_iter().enumerate() {
        world.set_block_with_data(
            base + glam::IVec3::new(0, 0, 3 * i as i32),
            Block::Log,
            data,
        );
    }
    for (i, data) in [ATTACH_POS_X, ATTACH_NEG_X, ATTACH_POS_Z, ATTACH_NEG_Z]
        .into_iter()
        .enumerate()
    {
        let at = base + glam::IVec3::new(4 + 2 * i as i32, 3, 0);
        world.set_block(at - glam::IVec3::Y, Block::Stone);
        world.set_block_with_data(at, Block::Torch, data);
    }
    // Ещё одно бревно в соседней секции (y 79 -> верх секции 4).
    world.set_block_with_data(base + glam::IVec3::new(1, 15, 1), Block::Log, AXIS_Z);
    light.add_column(&world, key);
    (world, light, key)
}

#[test]
fn oriented_blocks_survive_the_worker_round_trip() {
    let (world, light, key) = oriented_world();
    for ao in [true, false] {
        let options = MeshOptions {
            ambient_occlusion: ao,
        };
        let job = encode_column_job(&world, &light, 5, key, options).unwrap();
        let got = decode_result(&run_job(&job)).into_column();
        let want = mesh_world_column(&world, &light, key, options).unwrap();
        assert!(got.iter().zip(&want).all(|(a, b)| same(a, b)), "ao={ao}");
        // Данные действительно повлияли на меш: без них результат другой.
        let mut plain = World::new();
        for k in world.keys().collect::<Vec<_>>() {
            plain.insert(k, world.column(k).unwrap().clone());
        }
        for (pos, block) in [
            (key.origin() + glam::IVec3::new(4, 64, 4), Block::Log),
            (key.origin() + glam::IVec3::new(8, 67, 4), Block::Torch),
        ] {
            plain.set_block(pos, block);
        }
        let flat = mesh_world_column(&plain, &light, key, options).unwrap();
        assert!(!got.iter().zip(&flat).all(|(a, b)| same(a, b)), "ao={ao}");
    }
}

#[test]
fn section_job_carries_data_only_when_present() {
    let (world, light, key) = oriented_world();
    let options = MeshOptions::default();
    let with = encode_section_job(
        &world,
        &light,
        1,
        SectionPos::new(key.x(), 4, key.z()),
        options,
    )
    .unwrap();
    let without = encode_section_job(
        &world,
        &light,
        1,
        SectionPos::new(key.x(), 6, key.z()),
        options,
    );
    let without = without.unwrap();
    assert!(with.len() > without.len());
    assert_eq!(
        with.len() - 6,
        2 + 2 * super::CACHE_LEN + world::section::SECTION_VOLUME
    );
    let want = mesh_world_section(
        &world,
        &light,
        SectionPos::new(key.x(), 4, key.z()),
        options,
    )
    .unwrap();
    assert!(same(&decode_result(&run_job(&with)).into_section(), &want));
}
