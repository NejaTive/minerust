use super::expand::vertices;
use super::*;
use world::block::{COCOA_STAGE_SHIFT, FACING_NEG_Z, FACING_POS_X, FACING_POS_Z};

const AT: IVec3 = IVec3::new(5, 5, 5);

fn air(_: IVec3) -> Block {
    Block::Air
}

fn full(_: IVec3) -> u8 {
    MAX_LIGHT
}

fn mesh_cocoa(stage: u8, toward: u8) -> SectionMeshes {
    let mut section = Section::new();
    section.set_with_data(AT, Block::Cocoa, stage << COCOA_STAGE_SHIFT | toward);
    mesh_section(&section, air, full, MeshOptions::default())
}

fn extent(meshes: &SectionMeshes) -> ([f32; 3], [f32; 3]) {
    let (mut low, mut high) = ([f32::MAX; 3], [f32::MIN; 3]);
    for v in vertices(&meshes.cutout) {
        for axis in 0..3 {
            let at = (v.position()[axis] - AT[axis] as f32) * 16.0;
            low[axis] = low[axis].min(at);
            high[axis] = high[axis].max(at);
        }
    }
    (low, high)
}

#[test]
fn cocoa_is_a_cutout_pod_with_a_stem_in_the_texture_of_its_stage() {
    for (stage, tile) in [Tile::CocoaStage0, Tile::CocoaStage1, Tile::CocoaStage2]
        .into_iter()
        .enumerate()
    {
        let meshes = mesh_cocoa(stage as u8, FACING_POS_Z);
        assert!(meshes.solid.quads.is_empty());
        let verts = vertices(&meshes.cutout);
        assert_eq!(verts.len(), 4 * COCOA_PARTS as usize);
        assert!(verts.iter().all(|v| v.layer() == tile.layer() as u16));
    }
}

#[test]
fn cocoa_pod_is_pressed_against_the_trunk_it_faces() {
    let (low, high) = extent(&mesh_cocoa(2, FACING_POS_Z));
    assert_eq!((low[0], high[0]), (4.0, 12.0));
    assert_eq!((low[2], high[2]), (7.0, 15.0));
    assert_eq!((low[1], high[1]), (2.0, 16.0));
    let (low, high) = extent(&mesh_cocoa(2, FACING_POS_X));
    assert_eq!((low[0], high[0]), (7.0, 15.0));
    let (low, high) = extent(&mesh_cocoa(0, FACING_NEG_Z));
    assert_eq!((low[2], high[2]), (1.0, 5.0));
}
