use super::expand::vertices;
use super::*;

const AT: IVec3 = IVec3::new(5, 5, 5);

fn air(_: IVec3) -> Block {
    Block::Air
}

fn full(_: IVec3) -> u8 {
    MAX_LIGHT
}

fn mesh_lily() -> SectionMeshes {
    let mut section = Section::new();
    section.set(AT, Block::Waterlily);
    mesh_section(&section, air, full, MeshOptions::default())
}

#[test]
fn lily_is_a_flat_cutout_slab_a_sixteenth_above_the_cell_floor() {
    let meshes = mesh_lily();
    assert!(meshes.solid.quads.is_empty());
    let verts = vertices(&meshes.cutout);
    assert_eq!(verts.len(), 8);
    let heights: Vec<f32> = verts
        .iter()
        .map(|v| v.position()[1] - AT.y as f32)
        .collect();
    assert_eq!(heights.iter().filter(|&&h| h == 1.0 / 16.0).count(), 4);
    assert_eq!(heights.iter().filter(|&&h| h == 0.0).count(), 4);
    assert!(
        verts
            .iter()
            .all(|v| v.layer() == Tile::Waterlily.layer() as u16)
    );
}
