use glam::IVec3;

use mesh::mesh::quad_bytes;
use mesh::mesher::{MeshOptions, mesh_world_column};
use world::block::{ATTACH_NEG_X, ATTACH_NEG_Z, ATTACH_POS_X, ATTACH_POS_Z, AXIS_X, AXIS_Z, Block};
use world::column::{Column, WORLD_HEIGHT};
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
            let mut hash = FNV_START;
            for y in 0..WORLD_HEIGHT {
                for lz in 0..16 {
                    for lx in 0..16 {
                        let at = IVec3::new(lx, y, lz);
                        fnv(&mut hash, &[column.get(at) as u8, column.get_data(at)]);
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

const GOLDEN_ORIENTED: u64 = 0xeff95ad1fdd8c92b;
const GOLDEN_BLOCKS: [u64; 4] = [
    0x9abcf302903cbfa7,
    0xa8228d59edcc3d62,
    0x379f9328de36f1ee,
    0x0ac62a258542b464,
];
const GOLDEN_MESHES: [u64; 4] = [
    0xa94f0518d27d0c4a,
    0x92ef8c3c82ed38f5,
    0x2f6e2ca0accc1b54,
    0xbf0d189f2d53fcc5,
];
