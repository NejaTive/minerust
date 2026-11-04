use glam::IVec3;

use super::*;
use crate::block::{DOOR_OPEN, FACING_POS_X, FACING_POS_Z, GATE_OPEN};
use crate::column::Column;
use crate::pos::ColumnPos;

const FLOOR: i32 = 10;

fn world() -> World {
    let mut column = Column::new();
    for z in 0..16 {
        for x in 0..16 {
            column.set(IVec3::new(x, FLOOR, z), Block::Stone);
        }
    }
    let mut world = World::new();
    world.insert(ColumnPos::ZERO, column);
    world
}

fn at(x: i32, z: i32) -> IVec3 {
    IVec3::new(x, FLOOR + 1, z)
}

fn hinge(world: &World, pos: IVec3, look: IVec3, hit: (f32, f32)) -> bool {
    world.door_hinge_right(pos, Block::OakDoor, look, hit)
}

#[test]
fn a_door_places_both_halves_and_breaks_as_one() {
    let mut world = world();
    let pos = at(3, 3);
    assert!(world.can_place_door(pos, Block::OakDoor));
    world.place_door(pos, Block::OakDoor, FACING_POS_X | DOOR_PLACED_RIGHT);
    assert_eq!(world.block(pos), Block::OakDoor);
    assert_eq!(world.block_data(pos), FACING_POS_X);
    assert_eq!(world.block(pos + IVec3::Y), Block::OakDoorUpper);
    assert_eq!(world.block_data(pos + IVec3::Y), 1);

    let changed = world.set_block_supported(pos, Block::Air);
    assert!(changed.contains(&(pos + IVec3::Y)));
    assert_eq!(world.block(pos + IVec3::Y), Block::Air);

    world.place_door(pos, Block::OakDoor, FACING_POS_X);
    world.set_block_supported(pos + IVec3::Y, Block::Air);
    assert_eq!(world.block(pos), Block::Air);
}

#[test]
fn a_door_needs_room_for_two_blocks_and_a_floor() {
    let mut world = world();
    assert!(!world.can_place_door(at(2, 2) + IVec3::Y, Block::OakDoor));
    world.set_block(at(5, 5) + IVec3::Y, Block::Stone);
    assert!(!world.can_place_door(at(5, 5), Block::OakDoor));
    assert!(!world.can_place_door(at(5, 5) - IVec3::Y, Block::OakDoor));
}

#[test]
fn the_hinge_follows_the_click_point_when_nothing_is_around() {
    let world = world();
    let pos = at(5, 5);
    assert!(hinge(&world, pos, IVec3::X, (0.5, 0.8)));
    assert!(!hinge(&world, pos, IVec3::X, (0.5, 0.2)));
    assert!(hinge(&world, pos, IVec3::NEG_Z, (0.8, 0.5)));
    assert!(!hinge(&world, pos, IVec3::NEG_Z, (0.2, 0.5)));
}

#[test]
fn the_hinge_goes_to_the_walled_side() {
    let mut world = world();
    let pos = at(5, 5);
    world.set_block(pos + IVec3::Z, Block::Stone);
    assert!(
        hinge(&world, pos, IVec3::X, (0.5, 0.1)),
        "wall on the right"
    );
    world.set_block(pos + IVec3::Z, Block::Air);
    world.set_block(pos + IVec3::NEG_Z, Block::Stone);
    assert!(
        !hinge(&world, pos, IVec3::X, (0.5, 0.9)),
        "wall on the left"
    );
}

#[test]
fn a_neighbouring_door_gets_a_mirrored_hinge() {
    let mut world = world();
    world.place_door(at(5, 5), Block::OakDoor, FACING_POS_X);
    assert!(
        hinge(&world, at(5, 6), IVec3::X, (0.5, 0.1)),
        "door on the left"
    );
    assert!(
        !hinge(&world, at(5, 4), IVec3::X, (0.5, 0.9)),
        "door on the right"
    );
}

#[test]
fn hand_toggles_gates_and_wooden_doors_but_not_iron() {
    let mut world = world();
    let gate = at(2, 2);
    world.set_block_with_data(gate, Block::OakFenceGate, FACING_POS_X);
    world.toggle_by_hand(gate, IVec3::X);
    assert_eq!(world.block_data(gate), FACING_POS_X | GATE_OPEN);
    world.toggle_by_hand(gate, IVec3::X);
    assert_eq!(world.block_data(gate), FACING_POS_X);

    world.toggle_by_hand(gate, IVec3::NEG_X);
    assert_eq!(
        world.block_data(gate),
        crate::block::FACING_NEG_X | GATE_OPEN
    );

    let wood = at(6, 6);
    world.place_door(wood, Block::OakDoor, FACING_POS_Z);
    let changed = world.toggle_by_hand(wood + IVec3::Y, IVec3::Z);
    assert_eq!(changed, vec![wood, wood + IVec3::Y]);
    assert_eq!(world.block_data(wood), FACING_POS_Z | DOOR_OPEN);

    let iron = at(8, 8);
    world.place_door(iron, Block::IronDoor, FACING_POS_Z);
    assert!(world.toggle_by_hand(iron, IVec3::Z).is_empty());
    assert_eq!(world.block_data(iron), FACING_POS_Z);
}
