use super::*;
use crate::block::FACING_POS_X;
use crate::column::Column;
use crate::pos::ColumnPos;

const CENTER: IVec3 = IVec3::new(4, 12, 8);

fn room() -> World {
    let mut world = World::new();
    world.insert(ColumnPos::ZERO, Column::new());
    world.set_block(CENTER - IVec3::X, MARKER);
    for z in [CENTER.z - 1, CENTER.z + 1] {
        world.set_block(IVec3::new(CENTER.x - 1, CENTER.y, z), Block::Stone);
    }
    for z in 7..=9 {
        world.set_block_with_data(
            IVec3::new(CENTER.x, CENTER.y, z),
            Block::Lever,
            FACING_POS_X,
        );
    }
    let door = CENTER + IVec3::X * DOOR_DISTANCE;
    world.set_block(door, DOOR);
    world.set_block(door - IVec3::Y, DOOR);
    world
}

fn flip(world: &mut World, z: i32) -> Vec<IVec3> {
    let pos = IVec3::new(CENTER.x, CENTER.y, z);
    let data = world.block_data(pos) ^ LEVER_POWERED;
    world.set_block_with_data(pos, Block::Lever, data);
    world.solve_lever_puzzle(pos)
}

fn door_open(world: &World) -> bool {
    let door = CENTER + IVec3::X * DOOR_DISTANCE;
    world.block(door) == Block::Air && world.block(door - IVec3::Y) == Block::Air
}

#[test]
fn right_combination_opens_the_door_and_another_closes_it() {
    let mut world = room();
    assert!(flip(&mut world, 9).is_empty());
    assert!(!door_open(&world));
    assert_eq!(flip(&mut world, 7).len(), 2);
    assert!(door_open(&world));
    assert_eq!(flip(&mut world, 8).len(), 2);
    assert!(!door_open(&world));
    assert_eq!(flip(&mut world, 8).len(), 2);
    assert!(door_open(&world));
}

#[test]
fn levers_without_the_marker_do_nothing() {
    let mut world = room();
    world.set_block(CENTER - IVec3::X, Block::Stone);
    flip(&mut world, 9);
    flip(&mut world, 7);
    assert!(!door_open(&world));
}

#[test]
fn hand_toggle_reports_the_door_cells() {
    let mut world = room();
    world.toggle_by_hand(IVec3::new(CENTER.x, CENTER.y, 9), IVec3::X);
    let changed = world.toggle_by_hand(IVec3::new(CENTER.x, CENTER.y, 7), IVec3::X);
    assert_eq!(changed.len(), 3);
    assert!(door_open(&world));
}
