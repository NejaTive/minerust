use glam::IVec3;

use mesh::mesher::{MeshOptions, SectionMeshes, mesh_world_column};
use world::block::RenderLayer;
use world::light::LightMap;
use world::pos::ColumnPos;
use world::section::SECTION_SIZE;
use world::terrain::Terrain;
use world::world::World;

const KEYS: [(i32, i32); 4] = [(-4, -11), (6, -12), (0, -12), (0, -7)];

fn world_with_meshes() -> (World, Vec<(ColumnPos, Vec<SectionMeshes>)>) {
    let terrain = Terrain::new(1337);
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
    let meshes = KEYS
        .iter()
        .map(|&(x, z)| {
            let key = ColumnPos::new(x, z);
            let sections = mesh_world_column(&world, &light, key, MeshOptions::default())
                .expect("column is loaded");
            (key, sections)
        })
        .collect();
    (world, meshes)
}

#[test]
fn every_quad_lands_in_the_mesh_of_its_blocks_layer() {
    let (world, columns) = world_with_meshes();
    let (mut solid, mut cutout, mut water) = (0, 0, 0);
    for (key, sections) in &columns {
        for (sy, section) in sections.iter().enumerate() {
            let origin = key.origin() + IVec3::Y * (sy as i32 * SECTION_SIZE);
            for (mesh, layer) in [
                (&section.solid, RenderLayer::Opaque),
                (&section.cutout, RenderLayer::Cutout),
                (&section.water, RenderLayer::Translucent),
                (&section.ice, RenderLayer::Translucent),
            ] {
                for quad in &mesh.quads {
                    let [x, y, z] = quad.cell().map(i32::from);
                    let block = world.block(origin + IVec3::new(x, y, z));
                    assert_eq!(block.render_layer(), layer, "{block:?} at {key}");
                }
                match layer {
                    RenderLayer::Opaque => solid += mesh.quads.len(),
                    RenderLayer::Cutout => cutout += mesh.quads.len(),
                    RenderLayer::Translucent => water += mesh.quads.len(),
                }
            }
        }
    }
    assert!(
        solid > 0 && cutout > 0 && water > 0,
        "{solid} {cutout} {water}"
    );
}

fn quads_at(mesh: &mesh::mesher::MeshData, cell: [u8; 3]) -> usize {
    mesh.quads.iter().filter(|q| q.cell() == cell).count()
}

#[test]
fn lava_is_meshed_in_the_opaque_pass_and_exposes_neighbor_faces() {
    use world::block::Block;
    use world::column::Column;

    let mut world = World::new();
    for z in -1..=1 {
        for x in -1..=1 {
            let mut column = Column::new();
            for lz in 0..16 {
                for lx in 0..16 {
                    column.set(IVec3::new(lx, 20, lz), Block::Stone);
                }
            }
            world.insert(ColumnPos::new(x, z), column);
        }
    }
    world.set_block_with_data(IVec3::new(5, 21, 5), Block::Lava, 0);
    world.set_block_with_data(IVec3::new(6, 21, 5), Block::Lava, 2);
    world.set_block(IVec3::new(4, 21, 5), Block::Stone);
    let mut light = LightMap::new();
    let mut keys: Vec<ColumnPos> = world.keys().collect();
    keys.sort();
    for key in keys {
        light.add_column(&world, key);
    }
    let sections = mesh_world_column(&world, &light, ColumnPos::ZERO, MeshOptions::default())
        .expect("column is loaded");
    let section = &sections[1];
    assert!(section.water.quads.is_empty());
    assert_eq!(quads_at(&section.solid, [5, 5, 5]), 3);
    assert_eq!(quads_at(&section.solid, [6, 5, 5]), 4);
    assert_eq!(quads_at(&section.solid, [4, 5, 5]), 5);
}

#[test]
fn fluid_under_a_block_keeps_its_top_face_at_its_own_height() {
    use mesh::templates::{WATER_TOP, WATER_TOP_SLOPED};
    use world::block::{Block, FluidKind};
    use world::column::Column;

    for kind in [FluidKind::Water, FluidKind::Lava] {
        let mut world = World::new();
        for z in -1..=1 {
            for x in -1..=1 {
                let mut column = Column::new();
                for lz in 0..16 {
                    for lx in 0..16 {
                        column.set(IVec3::new(lx, 20, lz), Block::Stone);
                    }
                }
                world.insert(ColumnPos::new(x, z), column);
            }
        }
        for x in 4..9 {
            for z in 4..9 {
                world.set_block_with_data(IVec3::new(x, 21, z), kind.block(), 0);
            }
        }
        world.set_block(IVec3::new(6, 22, 6), Block::Stone);
        world.set_block_with_data(IVec3::new(6, 22, 7), kind.block(), 0);
        let mut light = LightMap::new();
        let mut keys: Vec<ColumnPos> = world.keys().collect();
        keys.sort();
        for key in keys {
            light.add_column(&world, key);
        }
        let sections = mesh_world_column(&world, &light, ColumnPos::ZERO, MeshOptions::default())
            .expect("column is loaded");
        let mesh = match kind {
            FluidKind::Water => &sections[1].water,
            FluidKind::Lava => &sections[1].solid,
        };
        let top = |x, z| {
            mesh.quads
                .iter()
                .filter(|q| {
                    q.cell() == [x, 5, z] && [WATER_TOP, WATER_TOP_SLOPED].contains(&q.template())
                })
                .count()
        };
        assert_eq!(top(6, 6), 1, "{kind:?} under stone");
        assert_eq!(top(6, 7), 0, "{kind:?} under the same fluid");
        assert_eq!(top(5, 6), 1, "{kind:?} open");
    }
}
