use super::*;
use crate::templates::POT_CROSS_PARTS;

const AT: IVec3 = IVec3::new(5, 5, 5);

fn air(_: IVec3) -> Block {
    Block::Air
}

fn full(_: IVec3) -> u8 {
    MAX_LIGHT
}

fn mesh(block: Block, data: u8) -> SectionMeshes {
    let mut section = Section::new();
    section.set_with_data(AT, block, data);
    mesh_section(&section, air, full, MeshOptions::default())
}

fn top_planes(meshes: &SectionMeshes, tile: Tile) -> Vec<f32> {
    meshes
        .cutout
        .quads
        .iter()
        .filter(|q| q.layer() == tile.layer() as u16 && q.template() == 0)
        .map(|q| f32::from(q.box_rect().1))
        .collect()
}

#[test]
fn cauldron_water_surface_follows_the_level() {
    assert!(top_planes(&mesh(Block::Cauldron, 0), Tile::Water).is_empty());
    for (level, plane) in [(1, 9.0), (2, 12.0), (3, 15.0)] {
        assert_eq!(
            top_planes(&mesh(Block::Cauldron, level), Tile::Water),
            [plane]
        );
    }
}

#[test]
fn cauldron_inside_uses_the_inner_texture() {
    let meshes = mesh(Block::Cauldron, 0);
    let inner = Tile::CauldronInner.layer() as u16;
    assert!(meshes.cutout.quads.iter().any(|q| q.layer() == inner));
}

#[test]
fn pot_plant_is_two_crossed_planes_in_the_plant_texture() {
    assert!(
        mesh(Block::FlowerPot, 0)
            .cutout
            .quads
            .iter()
            .all(|q| q.template() < crate::templates::POT_CROSS)
    );
    let meshes = mesh(Block::FlowerPot, Block::pot_code(Block::Poppy).unwrap());
    let planes: Vec<_> = meshes
        .cutout
        .quads
        .iter()
        .filter(|q| q.template() >= crate::templates::POT_CROSS)
        .collect();
    assert_eq!(planes.len(), usize::from(POT_CROSS_PARTS));
    assert!(
        planes
            .iter()
            .all(|q| q.layer() == Tile::Poppy.layer() as u16)
    );
    let cactus = mesh(Block::FlowerPot, Block::pot_code(Block::Cactus).unwrap());
    assert!(
        cactus
            .cutout
            .quads
            .iter()
            .any(|q| q.layer() == Tile::CactusSide.layer() as u16)
    );
}

#[test]
fn chest_faces_show_their_entity_tiles() {
    let meshes = mesh(Block::Chest, 0);
    let layers: std::collections::HashSet<u16> =
        meshes.cutout.quads.iter().map(|q| q.layer()).collect();
    for tile in [
        Tile::ChestFront,
        Tile::ChestBack,
        Tile::ChestTop,
        Tile::ChestBottom,
    ] {
        assert!(layers.contains(&(tile.layer() as u16)), "{tile:?}");
    }
}
