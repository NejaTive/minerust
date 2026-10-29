use glam::IVec3;

use mesh::mesh::quad_bytes;
use mesh::mesher::{MeshOptions, mesh_world_column};
use world::block::{ATTACH_NEG_X, ATTACH_NEG_Z, ATTACH_POS_X, ATTACH_POS_Z, AXIS_X, AXIS_Z, Block};
use world::column::{COLUMN_SECTIONS, Column, WORLD_HEIGHT};
use world::light::LightMap;
use world::pos::ColumnPos;
use world::terrain::Terrain;
use world::world::World;

const SEED: i32 = 1337;
const KEYS: [(i32, i32); 4] = [(-4, -11), (6, -12), (0, -12), (0, -7)];

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
            assert!((0..COLUMN_SECTIONS).all(|sy| column.section(sy).data().is_none()));
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

#[test]
fn oriented_scene_matches_golden() {
    let mut world = World::new();
    for z in -1..=1 {
        for x in -1..=1 {
            world.insert(ColumnPos::new(x, z), Column::new());
        }
    }
    for z in 0..16 {
        for x in 0..16 {
            world.set_block(IVec3::new(x, 9, z), Block::Stone);
        }
    }
    for x in 2..5 {
        world.set_block_with_data(IVec3::new(x, 10, 2), Block::Log, AXIS_X);
    }
    for z in 6..9 {
        world.set_block_with_data(IVec3::new(2, 10, z), Block::Log, AXIS_Z);
    }
    world.set_block(IVec3::new(8, 10, 8), Block::Log);
    world.set_block(IVec3::new(8, 11, 8), Block::Log);
    for (wall, data) in [
        (IVec3::new(11, 10, 5), ATTACH_NEG_X),
        (IVec3::new(11, 10, 9), ATTACH_POS_X),
        (IVec3::new(13, 10, 7), ATTACH_NEG_Z),
        (IVec3::new(9, 10, 7), ATTACH_POS_Z),
    ] {
        world.set_block(wall, Block::Stone);
        let torch = wall
            + match data {
                ATTACH_NEG_X => IVec3::X,
                ATTACH_POS_X => IVec3::NEG_X,
                ATTACH_NEG_Z => IVec3::Z,
                _ => IVec3::NEG_Z,
            };
        assert!(world.can_place_with_data(torch, Block::Torch, data));
        world.set_block_with_data(torch, Block::Torch, data);
    }
    world.set_block(IVec3::new(5, 10, 12), Block::Torch);
    let mut light = LightMap::new();
    let mut keys: Vec<ColumnPos> = world.keys().collect();
    keys.sort();
    for key in keys {
        light.add_column(&world, key);
    }
    let sections = mesh_world_column(&world, &light, ColumnPos::ZERO, MeshOptions::default())
        .expect("column is loaded");
    let mut hash = FNV_START;
    let column = world.column(ColumnPos::ZERO).expect("loaded");
    for y in 0..WORLD_HEIGHT {
        for z in 0..16 {
            for x in 0..16 {
                let pos = IVec3::new(x, y, z);
                fnv(&mut hash, &[column.get(pos) as u8, column.get_data(pos)]);
            }
        }
    }
    for section in &sections {
        for data in [&section.solid, &section.cutout, &section.water] {
            fnv(&mut hash, &quad_bytes(&data.quads));
        }
    }
    assert_eq!(hash, GOLDEN_ORIENTED, "oriented: {hash:#x}");
}

const GOLDEN_ORIENTED: u64 = 0xdf81dc4645d46957;
const GOLDEN_BLOCKS: [u64; 4] = [
    0xab405e38f0b2b8e5,
    0x6bc62c67f44d3de8,
    0xe69a701bfc6595d0,
    0x4fe7fff820d365db,
];
const GOLDEN_MESHES: [u64; 4] = [
    0xf8168e1e2bc0646c,
    0x6471aed5fe73aafe,
    0xbb91b5e8a5e32b86,
    0x20f612c7d843cfd2,
];
