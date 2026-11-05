use super::*;
use crate::block::{FACING_NEG_X, FACING_POS_X};
use crate::column::Column;

const FLOOR: i32 = 10;
const Y: i32 = FLOOR + 1;
const WEST: IVec3 = IVec3::new(5, Y, 8);
const EAST: IVec3 = IVec3::new(11, Y, 8);

fn world_with_line(from: i32, to: i32) -> World {
    let mut world = World::new();
    let mut column = Column::new();
    for lz in 0..16 {
        for lx in 0..16 {
            column.set(IVec3::new(lx, FLOOR, lz), Block::Stone);
        }
    }
    world.insert(ColumnPos::ZERO, column);
    for x in [from - 1, to + 1] {
        world.set_block(IVec3::new(x, Y, 8), Block::Stone);
    }
    world.set_block_with_data(IVec3::new(from, Y, 8), Block::TripwireHook, FACING_POS_X);
    world.set_block_with_data(IVec3::new(to, Y, 8), Block::TripwireHook, FACING_NEG_X);
    for x in from + 1..to {
        world.set_block(IVec3::new(x, Y, 8), Block::Tripwire);
    }
    world
}

fn standing(x: f32) -> Body {
    let at = Vec3::new(x, Y as f32, 8.5);
    (at - Vec3::new(0.3, 0.0, 0.3), at + Vec3::new(0.3, 1.8, 0.3))
}

fn powered(world: &World, hook: IVec3) -> bool {
    world.block_data(hook) & HOOK_POWERED != 0
}

#[test]
fn crossing_the_wire_powers_both_hooks() {
    let mut world = world_with_line(5, 11);
    let mut wires = Tripwires::new();
    let changed = wires.tick(&mut world, &[standing(8.5)]);
    assert_eq!(changed.len(), 2);
    assert!(powered(&world, WEST) && powered(&world, EAST));
}

#[test]
fn hooks_release_ten_ticks_after_everyone_leaves() {
    let mut world = world_with_line(5, 11);
    let mut wires = Tripwires::new();
    wires.tick(&mut world, &[standing(8.5)]);
    for _ in 0..RELEASE_TICKS - 1 {
        assert!(wires.tick(&mut world, &[]).is_empty());
    }
    assert_eq!(wires.tick(&mut world, &[]).len(), 2);
    assert!(!powered(&world, WEST) && !powered(&world, EAST));
}

#[test]
fn a_body_beside_the_wire_changes_nothing() {
    let mut world = world_with_line(5, 11);
    let mut wires = Tripwires::new();
    let beside = {
        let at = Vec3::new(8.5, Y as f32, 10.5);
        (at - Vec3::new(0.3, 0.0, 0.3), at + Vec3::new(0.3, 1.8, 0.3))
    };
    assert!(wires.tick(&mut world, &[beside]).is_empty());
}

#[test]
fn a_wire_with_a_missing_hook_is_slack() {
    let mut world = world_with_line(5, 11);
    world.set_block(EAST, Block::Air);
    let mut wires = Tripwires::new();
    assert!(wires.tick(&mut world, &[standing(8.5)]).is_empty());
}

#[test]
fn hooks_that_do_not_face_each_other_do_not_count() {
    let mut world = world_with_line(5, 11);
    world.set_block_with_data(EAST, Block::TripwireHook, FACING_POS_X);
    let mut wires = Tripwires::new();
    assert!(wires.tick(&mut world, &[standing(8.5)]).is_empty());
}

#[test]
fn the_line_may_hold_forty_wires_but_not_more() {
    let mut world = World::new();
    for key in [ColumnPos::ZERO, ColumnPos::new(1, 0), ColumnPos::new(2, 0)] {
        world.insert(key, Column::new());
    }
    let place = |world: &mut World, from: i32, to: i32| {
        world.set_block_with_data(IVec3::new(from, Y, 8), Block::TripwireHook, FACING_POS_X);
        world.set_block_with_data(IVec3::new(to, Y, 8), Block::TripwireHook, FACING_NEG_X);
        for x in from + 1..to {
            world.set_block(IVec3::new(x, Y, 8), Block::Tripwire);
        }
    };
    place(&mut world, 2, 43);
    assert!(line_ends(&world, IVec3::new(20, Y, 8)).is_some());
    world.set_block(IVec3::new(43, Y, 8), Block::Air);
    place(&mut world, 2, 44);
    assert!(line_ends(&world, IVec3::new(20, Y, 8)).is_none());
}

#[test]
fn adopted_powered_hook_is_released() {
    let mut world = world_with_line(5, 11);
    world.set_block_with_data(WEST, Block::TripwireHook, FACING_POS_X | HOOK_POWERED);
    let mut wires = Tripwires::new();
    wires.adopt_column(ColumnPos::ZERO, world.column(ColumnPos::ZERO).unwrap());
    for _ in 0..RELEASE_TICKS {
        wires.tick(&mut world, &[]);
    }
    assert!(!powered(&world, WEST));
}
