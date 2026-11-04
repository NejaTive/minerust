use glam::Vec3;
use world::block::{RAIL_ASCENDING_EAST, RAIL_NORTH_SOUTH, RAIL_SOUTH_EAST};

use super::expand::vertices;
use super::*;

const AT: IVec3 = IVec3::new(5, 5, 5);

fn air(_: IVec3) -> Block {
    Block::Air
}

fn full(_: IVec3) -> u8 {
    MAX_LIGHT
}

fn mesh_with(blocks: &[(IVec3, Block, u8)]) -> SectionMeshes {
    let mut section = Section::new();
    for &(pos, block, data) in blocks {
        section.set_with_data(pos, block, data);
    }
    mesh_section(&section, air, full, MeshOptions::default())
}

fn bounds(quads: &MeshData) -> (Vec3, Vec3) {
    let points: Vec<Vec3> = vertices(quads)
        .iter()
        .map(|v| Vec3::from(v.position()))
        .collect();
    (
        points.iter().copied().reduce(Vec3::min).unwrap(),
        points.iter().copied().reduce(Vec3::max).unwrap(),
    )
}

#[test]
fn neighbouring_panes_do_not_draw_the_shared_end_faces() {
    let single = mesh_with(&[(AT, Block::GlassPane, 0)]);
    let pair = mesh_with(&[
        (AT, Block::GlassPane, 0),
        (AT + IVec3::X, Block::GlassPane, 0),
    ]);
    assert!(single.solid.quads.is_empty());
    assert!(pair.cutout.quads.len() < 2 * single.cutout.quads.len());
}

#[test]
fn a_pane_next_to_a_full_block_loses_the_face_against_it() {
    let alone = mesh_with(&[(AT, Block::IronBars, 0)]);
    let by_stone = mesh_with(&[(AT, Block::IronBars, 0), (AT + IVec3::X, Block::Stone, 0)]);
    let stone_faces = 5;
    assert!(by_stone.cutout.quads.len() < alone.cutout.quads.len() + stone_faces);
    let (_, max) = bounds(&by_stone.cutout);
    assert!(max.x <= AT.x as f32 + 1.0);
}

#[test]
fn a_fence_grows_arms_towards_a_neighbour_fence() {
    let alone = mesh_with(&[(AT, Block::OakFence, 0)]);
    let pair = mesh_with(&[
        (AT, Block::OakFence, 0),
        (AT + IVec3::X, Block::OakFence, 0),
    ]);
    assert_eq!(alone.solid.quads.len(), 6);
    let (min, max) = bounds(&pair.solid);
    assert_eq!(min.x, AT.x as f32 + 6.0 / 16.0);
    assert_eq!(max.x, AT.x as f32 + 1.0 + 10.0 / 16.0);
}

#[test]
fn a_door_draws_a_thin_cutout_slab_over_both_halves() {
    let meshes = mesh_with(&[
        (AT, Block::OakDoor, world::block::FACING_POS_X),
        (AT + IVec3::Y, Block::OakDoorUpper, 0),
    ]);
    let (min, max) = bounds(&meshes.cutout);
    assert_eq!((min.y, max.y), (AT.y as f32, AT.y as f32 + 2.0));
    assert_eq!((min.x, max.x), (AT.x as f32, AT.x as f32 + 3.0 / 16.0));
}

#[test]
fn rails_are_a_single_flat_quad_a_sixteenth_up() {
    let meshes = mesh_with(&[(AT, Block::Rail, RAIL_NORTH_SOUTH)]);
    assert_eq!(meshes.cutout.quads.len(), 1);
    let (min, max) = bounds(&meshes.cutout);
    assert_eq!(
        (min.y, max.y),
        (AT.y as f32 + 1.0 / 16.0, AT.y as f32 + 1.0 / 16.0)
    );
    assert_eq!((max - min).x, 1.0);
}

#[test]
fn a_climbing_rail_rises_a_block_on_one_side() {
    let meshes = mesh_with(&[(AT, Block::Rail, RAIL_ASCENDING_EAST)]);
    let (min, max) = bounds(&meshes.cutout);
    assert_eq!(max.y - min.y, 1.0);
}

#[test]
fn curved_rails_use_the_turned_texture() {
    let straight = vertices(&mesh_with(&[(AT, Block::Rail, RAIL_NORTH_SOUTH)]).cutout);
    let curve = vertices(&mesh_with(&[(AT, Block::Rail, RAIL_SOUTH_EAST)]).cutout);
    assert_eq!(straight[0].layer(), Tile::RailNormal.layer() as u16);
    assert_eq!(curve[0].layer(), Tile::RailTurned.layer() as u16);
}

#[test]
fn a_web_is_a_cutout_cross() {
    let meshes = mesh_with(&[(AT, Block::Web, 0)]);
    assert_eq!(meshes.cutout.quads.len(), 4);
    assert!(meshes.solid.quads.is_empty());
}
