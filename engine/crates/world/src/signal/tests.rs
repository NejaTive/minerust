use super::*;
use crate::block::PLATE_POWERED;
use crate::column::Column;
use crate::pos::ColumnPos;

const FLOOR: i32 = 10;

fn flat_world() -> World {
    let mut world = World::new();
    let mut column = Column::new();
    for lz in 0..16 {
        for lx in 0..16 {
            column.set(IVec3::new(lx, FLOOR, lz), Block::Stone);
        }
    }
    world.insert(ColumnPos::ZERO, column);
    world
}

fn at(x: i32, y: i32, z: i32) -> IVec3 {
    IVec3::new(x, y, z)
}

fn press(world: &mut World, pos: IVec3) {
    world.set_block_with_data(pos, Block::StonePressurePlate, PLATE_POWERED);
}

#[test]
fn unpressed_plate_gives_no_power() {
    let mut world = flat_world();
    world.set_block(at(8, FLOOR + 1, 8), Block::StonePressurePlate);
    world.set_block(at(9, FLOOR, 8), Block::Tnt);
    assert_eq!(power(&world, at(9, FLOOR, 8)), 0);
}

#[test]
fn pressed_plate_strongly_powers_the_block_below() {
    let mut world = flat_world();
    let plate = at(8, FLOOR + 1, 8);
    press(&mut world, plate);
    let below = plate - IVec3::Y;
    assert_eq!(strong_power_into(&world, below), MAX_POWER);
    world.set_block(at(9, FLOOR, 8), Block::Tnt);
    assert_eq!(power(&world, at(9, FLOOR, 8)), MAX_POWER);
}

#[test]
fn pressed_plate_powers_tnt_next_to_it_weakly() {
    let mut world = flat_world();
    press(&mut world, at(8, FLOOR + 1, 8));
    world.set_block(at(9, FLOOR + 1, 8), Block::Tnt);
    assert_eq!(power(&world, at(9, FLOOR + 1, 8)), MAX_POWER);
}

#[test]
fn power_does_not_reach_past_the_powered_block() {
    let mut world = flat_world();
    press(&mut world, at(8, FLOOR + 1, 8));
    world.set_block(at(10, FLOOR, 8), Block::Tnt);
    world.set_block(at(9, FLOOR + 1, 9), Block::Tnt);
    assert_eq!(power(&world, at(10, FLOOR, 8)), 0);
    assert_eq!(power(&world, at(9, FLOOR + 1, 9)), 0);
}

#[test]
fn power_does_not_pass_through_a_non_solid_block() {
    let mut world = flat_world();
    world.set_block(at(8, FLOOR, 8), Block::Glass);
    press(&mut world, at(8, FLOOR + 1, 8));
    world.set_block(at(9, FLOOR, 8), Block::Tnt);
    assert_eq!(power(&world, at(9, FLOOR, 8)), 0);
}

#[test]
fn signal_ignites_tnt_next_to_a_powered_block() {
    let mut world = flat_world();
    let mut signal = Signal::new();
    let tnt = at(9, FLOOR, 8);
    world.set_block(tnt, Block::Tnt);
    let plate = at(8, FLOOR + 1, 8);
    press(&mut world, plate);
    signal.block_changed(&world, plate);
    let ignition = signal.step(&mut world);
    assert_eq!(ignition.ignited, vec![tnt]);
    assert_eq!(world.block(tnt), Block::Air);
    assert!(signal.step(&mut world).ignited.is_empty());
    assert!(signal.is_empty());
}

#[test]
fn placing_tnt_next_to_a_source_ignites_it() {
    let mut world = flat_world();
    let mut signal = Signal::new();
    press(&mut world, at(8, FLOOR + 1, 8));
    let tnt = at(8, FLOOR, 9);
    world.set_block(tnt, Block::Tnt);
    signal.block_changed(&world, tnt);
    assert_eq!(signal.step(&mut world).ignited, vec![tnt]);
}

#[test]
fn unpowered_tnt_stays() {
    let mut world = flat_world();
    let mut signal = Signal::new();
    let tnt = at(9, FLOOR, 8);
    world.set_block(tnt, Block::Tnt);
    signal.block_changed(&world, tnt);
    assert!(signal.step(&mut world).ignited.is_empty());
    assert_eq!(world.block(tnt), Block::Tnt);
}
