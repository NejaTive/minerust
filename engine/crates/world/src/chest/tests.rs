use glam::IVec3;

use super::*;
use crate::block::Block;
use crate::column::Column;
use crate::pos::ColumnPos;
use crate::save::{decode_column, encode_column};
use crate::textures::TextureSet;
use crate::world::World;

fn world() -> World {
    let mut world = World::new();
    world.insert(ColumnPos::ZERO, Column::new());
    world
}

fn place(world: &mut World, pos: IVec3, look: IVec3) -> Option<u8> {
    let data = world.chest_placement_data(pos, look)?;
    world.set_block_with_data(pos, Block::Chest, data);
    Some(data)
}

#[test]
fn two_neighbouring_chests_become_one_double() {
    let mut world = world();
    let (a, b) = (IVec3::new(4, 10, 4), IVec3::new(5, 10, 4));
    place(&mut world, a, IVec3::Z).unwrap();
    place(&mut world, b, IVec3::Z).unwrap();
    assert_eq!(world.chest_partner(a), Some(b));
    assert_eq!(world.chest_partner(b), Some(a));
    assert_eq!(world.chest_cells(b), vec![a, b]);
    assert_eq!(world.chest_contents(a).len(), 2 * CHEST_SLOTS);
}

#[test]
fn a_third_chest_is_refused() {
    let mut world = world();
    for x in [4, 5] {
        place(&mut world, IVec3::new(x, 10, 4), IVec3::Z).unwrap();
    }
    assert_eq!(
        world.chest_placement_data(IVec3::new(6, 10, 4), IVec3::Z),
        None
    );
    assert_eq!(
        world.chest_placement_data(IVec3::new(3, 10, 4), IVec3::Z),
        None
    );
}

#[test]
fn a_chest_placed_beside_another_adopts_its_facing() {
    let mut world = world();
    let a = IVec3::new(4, 10, 4);
    place(&mut world, a, IVec3::Z).unwrap();
    let data = world
        .chest_placement_data(IVec3::new(5, 10, 4), IVec3::X)
        .unwrap();
    assert_eq!(front_of(data), IVec3::NEG_Z);
}

#[test]
fn breaking_one_half_leaves_a_single_chest() {
    let mut world = world();
    let (a, b) = (IVec3::new(4, 10, 4), IVec3::new(5, 10, 4));
    place(&mut world, a, IVec3::Z).unwrap();
    place(&mut world, b, IVec3::Z).unwrap();
    world.set_block(a, Block::Air);
    assert_eq!(world.chest_partner(b), None);
    assert_eq!(half_of(world.block_data(b)), HALF_NONE);
    assert_eq!(world.chest_contents(b).len(), CHEST_SLOTS);
}

#[test]
fn the_double_chest_keeps_the_lower_coordinate_chest_on_top() {
    let mut world = world();
    let (a, b) = (IVec3::new(4, 10, 4), IVec3::new(5, 10, 4));
    place(&mut world, a, IVec3::Z).unwrap();
    place(&mut world, b, IVec3::Z).unwrap();
    assert!(world.set_chest_slot(b, 0, Some("stone".into())));
    assert!(world.set_chest_slot(b, CHEST_SLOTS, Some("dirt".into())));
    let from_a = world.chest_contents(a);
    assert_eq!(from_a[0].as_deref(), Some("stone"));
    assert_eq!(from_a[CHEST_SLOTS].as_deref(), Some("dirt"));
    assert_eq!(world.chest_contents(b), from_a);
    world.set_block(b, Block::Air);
    assert_eq!(world.chest_contents(a)[0].as_deref(), Some("stone"));
}

#[test]
fn contents_are_created_lazily_and_empty() {
    let mut world = world();
    let pos = IVec3::new(2, 20, 2);
    world.set_block_with_data(pos, Block::Chest, data(IVec3::NEG_Z, HALF_NONE));
    let column = world.column(ColumnPos::ZERO).unwrap();
    assert_eq!(column.chests().count(), 0);
    assert_eq!(world.chest_contents(pos), vec![None; CHEST_SLOTS]);
    assert!(world.set_chest_slot(pos, 3, Some("stone".into())));
    assert_eq!(world.column(ColumnPos::ZERO).unwrap().chests().count(), 1);
}

#[test]
fn breaking_a_chest_deletes_its_contents() {
    let mut world = world();
    let pos = IVec3::new(2, 20, 2);
    world.set_block_with_data(pos, Block::Chest, 0);
    world.set_chest_slot(pos, 0, Some("stone".into()));
    world.set_block(pos, Block::Air);
    world.set_block_with_data(pos, Block::Chest, 0);
    assert_eq!(world.chest_contents(pos), vec![None; CHEST_SLOTS]);
}

#[test]
fn contents_survive_the_save() {
    let mut world = world();
    let pos = IVec3::new(2, 20, 7);
    world.set_block_with_data(pos, Block::Chest, 2);
    world.set_chest_slot(pos, 5, Some("red_wool".into()));
    world.set_chest_slot(pos, 26, Some("poppy".into()));
    let column = world.column(ColumnPos::ZERO).unwrap();
    let back = decode_column(&encode_column(column)).expect("decodes");
    let mut loaded = World::new();
    loaded.insert(ColumnPos::ZERO, back);
    let slots = loaded.chest_contents(pos);
    assert_eq!(slots[5].as_deref(), Some("red_wool"));
    assert_eq!(slots[26].as_deref(), Some("poppy"));
    assert_eq!(slots.iter().flatten().count(), 2);
}

#[test]
fn an_opaque_block_above_blocks_the_chest() {
    let mut world = world();
    let pos = IVec3::new(2, 20, 2);
    world.set_block_with_data(pos, Block::Chest, 0);
    assert!(!world.chest_blocked(pos));
    world.set_block(pos + IVec3::Y, Block::Stone);
    assert!(world.chest_blocked(pos));
    world.set_block(pos + IVec3::Y, Block::Glass);
    assert!(!world.chest_blocked(pos));
}

#[test]
fn a_double_chest_is_blocked_by_either_half() {
    let mut world = world();
    let (a, b) = (IVec3::new(4, 10, 4), IVec3::new(5, 10, 4));
    place(&mut world, a, IVec3::Z).unwrap();
    place(&mut world, b, IVec3::Z).unwrap();
    world.set_block(b + IVec3::Y, Block::Stone);
    assert!(world.chest_blocked(a));
}

#[test]
fn faces_map_to_tiles_by_the_front_direction() {
    let facing_south = data(IVec3::Z, HALF_NONE);
    assert_eq!(tile_for(facing_south, IVec3::Z), Tile::ChestFront);
    assert_eq!(tile_for(facing_south, IVec3::NEG_Z), Tile::ChestBack);
    assert_eq!(tile_for(facing_south, IVec3::Y), Tile::ChestTop);
    let left = right_of(IVec3::Z);
    assert_eq!(tile_for(facing_south, left), Tile::ChestRight);
    let double = data(IVec3::NEG_Z, HALF_RIGHT);
    assert_eq!(tile_for(double, IVec3::NEG_Z), Tile::ChestDoubleFrontR);
    assert_eq!(
        tile_for(double, right_of(IVec3::NEG_Z)),
        Tile::ChestDoubleEndR
    );
}

#[test]
fn every_chest_tile_is_painted_where_the_boxes_are() {
    let set = TextureSet::standard();
    for (_, tile) in SINGLE_TILES {
        let pixels = set.tile_pixels(tile);
        let opaque = pixels.chunks(4).filter(|p| p[3] == 255).count();
        assert!(opaque > 0, "{tile:?}");
    }
    for (tile, ..) in DOUBLE_TILES {
        let pixels = set.tile_pixels(tile);
        assert!(pixels.chunks(4).any(|p| p[3] == 255), "{tile:?}");
    }
}

#[test]
fn the_front_shows_the_lid_and_the_body_with_the_latch() {
    let set = TextureSet::standard();
    let size = set.size() as usize;
    let pixels = set.tile_pixels(Tile::ChestFront);
    let alpha = |u: usize, v: usize| pixels[(v * size + u) * 4 + 3];
    let at = |units: f32| (units * size as f32 / 16.0) as usize;
    assert_eq!(alpha(at(8.0), at(16.0 - 12.0)), 255);
    assert_eq!(alpha(at(8.0), at(16.0 - 8.0)), 255);
    assert_eq!(alpha(at(0.5), at(8.0)), 0);
    assert_eq!(alpha(at(8.0), at(0.5)), 0);
}
