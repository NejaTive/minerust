use glam::IVec3;

use super::*;
use crate::block::{
    RAIL_ASCENDING_EAST, RAIL_ASCENDING_NORTH, RAIL_EAST_WEST, RAIL_NORTH_EAST, RAIL_NORTH_SOUTH,
    RAIL_SOUTH_EAST,
};
use crate::column::Column;
use crate::pos::ColumnPos;

const FLOOR: i32 = 10;

fn world() -> World {
    let mut column = Column::new();
    for y in 0..=FLOOR + 1 {
        for z in 0..16 {
            for x in 0..16 {
                column.set(IVec3::new(x, y, z), Block::Stone);
            }
        }
    }
    let mut world = World::new();
    world.insert(ColumnPos::ZERO, column);
    for z in 0..16 {
        for x in 0..16 {
            world.set_block(IVec3::new(x, FLOOR + 1, z), Block::Air);
        }
    }
    world
}

fn at(x: i32, z: i32) -> IVec3 {
    IVec3::new(x, FLOOR + 1, z)
}

fn lay(world: &mut World, pos: IVec3) -> Vec<IVec3> {
    world.set_block_with_data(pos, Block::Rail, 0);
    world.connect_rail(pos)
}

fn shape(world: &World, pos: IVec3) -> u8 {
    world.block_data(pos)
}

#[test]
fn a_lone_rail_runs_north_south() {
    let mut world = world();
    lay(&mut world, at(5, 5));
    assert_eq!(shape(&world, at(5, 5)), RAIL_NORTH_SOUTH);
}

#[test]
fn a_neighbour_turns_the_old_rail_to_face_it() {
    let mut world = world();
    lay(&mut world, at(5, 5));
    let changed = lay(&mut world, at(6, 5));
    assert_eq!(shape(&world, at(6, 5)), RAIL_EAST_WEST);
    assert_eq!(shape(&world, at(5, 5)), RAIL_EAST_WEST);
    assert!(changed.contains(&at(5, 5)));
}

#[test]
fn a_rail_to_the_south_of_an_east_west_line_becomes_a_curve() {
    let mut world = world();
    lay(&mut world, at(5, 5));
    lay(&mut world, at(6, 5));
    lay(&mut world, at(5, 6));
    assert_eq!(shape(&world, at(5, 6)), RAIL_NORTH_SOUTH);
    assert_eq!(shape(&world, at(5, 5)), RAIL_SOUTH_EAST);
}

#[test]
fn a_curve_closes_between_a_northern_and_an_eastern_neighbour() {
    let mut world = world();
    lay(&mut world, at(5, 4));
    lay(&mut world, at(6, 5));
    lay(&mut world, at(5, 5));
    assert_eq!(shape(&world, at(5, 5)), RAIL_NORTH_EAST);
}

#[test]
fn a_rail_climbs_towards_a_rail_one_block_higher() {
    let mut world = world();
    world.set_block(at(5, 4), Block::Stone);
    lay(&mut world, at(5, 4) + IVec3::Y);
    lay(&mut world, at(5, 5));
    assert_eq!(shape(&world, at(5, 5)), RAIL_ASCENDING_NORTH);
    assert_eq!(shape(&world, at(5, 4) + IVec3::Y), RAIL_NORTH_SOUTH);

    lay(&mut world, at(8, 5));
    lay(&mut world, at(9, 5) + IVec3::Y);
    world.set_block(at(9, 5), Block::Stone);
    lay(&mut world, at(7, 5));
    assert_eq!(shape(&world, at(8, 5)), RAIL_ASCENDING_EAST);
}

#[test]
fn a_rail_without_support_is_dropped() {
    let mut world = world();
    lay(&mut world, at(5, 5));
    let changed = world.set_block_supported(at(5, 5) + IVec3::NEG_Y, Block::Air);
    assert!(changed.contains(&at(5, 5)));
    assert_eq!(world.block(at(5, 5)), Block::Air);
    assert!(!world.can_place_with_data(at(5, 5) + IVec3::Y, Block::Rail, 0));
}
