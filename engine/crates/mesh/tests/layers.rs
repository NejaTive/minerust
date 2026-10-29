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
