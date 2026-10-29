use super::*;
use crate::mesher::{mesh_world_column, mesh_world_section};
use world::block::Block;
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
