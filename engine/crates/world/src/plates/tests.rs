use super::*;
use crate::column::Column;
use crate::pos::ColumnPos;

const FLOOR: i32 = 10;
const PLATE: IVec3 = IVec3::new(8, FLOOR + 1, 8);

fn world_with_plate() -> World {
    let mut world = World::new();
    let mut column = Column::new();
    for lz in 0..16 {
        for lx in 0..16 {
            column.set(IVec3::new(lx, FLOOR, lz), Block::Stone);
        }
    }
    world.insert(ColumnPos::ZERO, column);
    world.set_block(PLATE, Block::StonePressurePlate);
    world
}

fn standing(at: Vec3) -> Body {
    (at - Vec3::new(0.3, 0.0, 0.3), at + Vec3::new(0.3, 1.8, 0.3))
}

fn on_plate() -> Body {
    standing(Vec3::new(8.5, FLOOR as f32 + 1.0, 8.5))
}

fn powered(world: &World) -> bool {
    world.block_data(PLATE) & PLATE_POWERED != 0
}

#[test]
fn body_on_the_plate_presses_it() {
    let mut world = world_with_plate();
    let mut plates = Plates::new();
    assert_eq!(plates.tick(&mut world, &[on_plate()]), vec![PLATE]);
    assert!(powered(&world));
    assert!(plates.tick(&mut world, &[on_plate()]).is_empty());
}

#[test]
fn body_beside_the_plate_does_not_press_it() {
    let mut world = world_with_plate();
    let mut plates = Plates::new();
    let beside = standing(Vec3::new(9.5, FLOOR as f32 + 1.0, 8.5));
    assert!(plates.tick(&mut world, &[beside]).is_empty());
    assert!(!powered(&world));
}

#[test]
fn body_above_the_sensitive_volume_does_not_press_it() {
    let mut world = world_with_plate();
    let mut plates = Plates::new();
    let airborne = standing(Vec3::new(8.5, FLOOR as f32 + 1.3, 8.5));
    assert!(plates.tick(&mut world, &[airborne]).is_empty());
}

#[test]
fn plate_releases_twenty_ticks_after_the_body_leaves() {
    let mut world = world_with_plate();
    let mut plates = Plates::new();
    plates.tick(&mut world, &[on_plate()]);
    for _ in 0..RELEASE_TICKS - 1 {
        assert!(plates.tick(&mut world, &[]).is_empty());
        assert!(powered(&world));
    }
    assert_eq!(plates.tick(&mut world, &[]), vec![PLATE]);
    assert!(!powered(&world));
}

#[test]
fn returning_body_restarts_the_release_timer() {
    let mut world = world_with_plate();
    let mut plates = Plates::new();
    plates.tick(&mut world, &[on_plate()]);
    for _ in 0..RELEASE_TICKS - 1 {
        plates.tick(&mut world, &[]);
    }
    plates.tick(&mut world, &[on_plate()]);
    for _ in 0..RELEASE_TICKS - 1 {
        plates.tick(&mut world, &[]);
    }
    assert!(powered(&world));
}

#[test]
fn adopted_pressed_plate_is_released_when_nobody_stands_on_it() {
    let mut world = world_with_plate();
    world.set_block_with_data(PLATE, Block::StonePressurePlate, PLATE_POWERED);
    let mut plates = Plates::new();
    plates.adopt_column(ColumnPos::ZERO, world.column(ColumnPos::ZERO).unwrap());
    for _ in 0..RELEASE_TICKS - 1 {
        assert!(plates.tick(&mut world, &[]).is_empty());
    }
    assert_eq!(plates.tick(&mut world, &[]), vec![PLATE]);
    assert!(!powered(&world));
}

#[test]
fn adopted_pressed_plate_stays_under_a_body() {
    let mut world = world_with_plate();
    world.set_block_with_data(PLATE, Block::StonePressurePlate, PLATE_POWERED);
    let mut plates = Plates::new();
    plates.adopt_column(ColumnPos::ZERO, world.column(ColumnPos::ZERO).unwrap());
    for _ in 0..RELEASE_TICKS * 3 {
        plates.tick(&mut world, &[on_plate()]);
    }
    assert!(powered(&world));
}
