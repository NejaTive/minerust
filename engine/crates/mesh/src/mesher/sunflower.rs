use super::expand::vertices;
use super::*;

const AT: IVec3 = IVec3::new(5, 5, 5);

fn air(_: IVec3) -> Block {
    Block::Air
}

fn full(_: IVec3) -> u8 {
    MAX_LIGHT
}

fn mesh_of(block: Block) -> SectionMeshes {
    let mut section = Section::new();
    section.set(AT, block);
    mesh_section(&section, air, full, MeshOptions::default())
}

#[test]
fn sunflower_top_is_a_cross_with_a_head_on_the_east_side() {
    let meshes = mesh_of(Block::SunflowerUpper);
    let verts = vertices(&meshes.cutout);
    assert_eq!(verts.len(), 6 * 4);
    let layers = |tile: Tile| {
        verts
            .iter()
            .filter(|v| v.layer() == tile.layer() as u16)
            .count()
    };
    assert_eq!(layers(Tile::SunflowerTop), 16);
    assert_eq!(layers(Tile::SunflowerFront), 4);
    assert_eq!(layers(Tile::SunflowerBack), 4);
    let head_x: Vec<f32> = verts
        .iter()
        .filter(|v| v.layer() == Tile::SunflowerFront.layer() as u16)
        .map(|v| v.position()[0] - AT.x as f32)
        .collect();
    assert!(head_x.iter().all(|&x| x == 10.0 / 16.0), "{head_x:?}");
}

#[test]
fn other_double_plant_halves_are_plain_crosses() {
    for block in [
        Block::SunflowerLower,
        Block::LilacUpper,
        Block::DoubleFernLower,
    ] {
        let verts = vertices(&mesh_of(block).cutout);
        assert_eq!(verts.len(), 16, "{block:?}");
    }
}
