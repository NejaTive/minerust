use super::*;
use crate::block::FLUID_SOURCE;
use crate::column::Column;
use crate::pos::ColumnPos;

const FLOOR: i32 = 10;

fn flat_world(columns: &[(i32, i32)]) -> World {
    let mut world = World::new();
    for &(x, z) in columns {
        let mut column = Column::new();
        for lz in 0..16 {
            for lx in 0..16 {
                column.set(IVec3::new(lx, FLOOR, lz), Block::Stone);
            }
        }
        world.insert(ColumnPos::new(x, z), column);
    }
    world
}

fn at(y: i32) -> IVec3 {
    IVec3::new(8, y, 8)
}

fn put(world: &mut World, falling: &mut Falling, pos: IVec3, block: Block) {
    world.set_block_supported(pos, block);
    falling.block_changed(world, pos);
}

fn settle(world: &mut World, falling: &mut Falling) {
    for _ in 0..200 {
        if falling.is_empty() {
            return;
        }
        falling.step(world, |_| false);
    }
    panic!("falling does not settle");
}

#[test]
fn sand_falls_to_the_floor() {
    let mut world = flat_world(&[(0, 0)]);
    let mut falling = Falling::new();
    put(&mut world, &mut falling, at(20), Block::Sand);
    settle(&mut world, &mut falling);
    assert_eq!(world.block(at(FLOOR + 1)), Block::Sand);
    for y in FLOOR + 2..=20 {
        assert_eq!(world.block(at(y)), Block::Air, "y {y}");
    }
}

#[test]
fn falls_one_cell_per_step() {
    let mut world = flat_world(&[(0, 0)]);
    let mut falling = Falling::new();
    put(&mut world, &mut falling, at(20), Block::Gravel);
    let changed = falling.step(&mut world, |_| false);
    assert_eq!(changed, [at(19), at(20)]);
    assert_eq!(world.block(at(19)), Block::Gravel);
    assert_eq!(world.block(at(20)), Block::Air);
}

#[test]
fn column_of_three_collapses() {
    let mut world = flat_world(&[(0, 0)]);
    let mut falling = Falling::new();
    for y in 14..=16 {
        put(&mut world, &mut falling, at(y), Block::Sand);
    }
    settle(&mut world, &mut falling);
    for y in FLOOR + 1..=FLOOR + 3 {
        assert_eq!(world.block(at(y)), Block::Sand, "y {y}");
    }
    assert_eq!(world.block(at(FLOOR + 4)), Block::Air);
    assert_eq!(world.block(at(16)), Block::Air);
}

#[test]
fn sand_on_solid_stays() {
    let mut world = flat_world(&[(0, 0)]);
    let mut falling = Falling::new();
    put(&mut world, &mut falling, at(FLOOR + 1), Block::Sand);
    assert!(falling.step(&mut world, |_| false).is_empty());
    assert_eq!(world.block(at(FLOOR + 1)), Block::Sand);
}

#[test]
fn displaces_water_of_any_level() {
    for data in [FLUID_SOURCE, 3, crate::block::FLUID_FALLING] {
        let mut world = flat_world(&[(0, 0)]);
        let mut falling = Falling::new();
        world.set_block_with_data(at(FLOOR + 1), Block::Water, data);
        put(&mut world, &mut falling, at(FLOOR + 2), Block::Sand);
        settle(&mut world, &mut falling);
        assert_eq!(world.block(at(FLOOR + 1)), Block::Sand, "data {data}");
        assert_eq!(world.block_data(at(FLOOR + 1)), 0);
        assert_eq!(world.block(at(FLOOR + 2)), Block::Air);
    }
}

#[test]
fn breaks_flower_and_torch_below() {
    for plant in [Block::Poppy, Block::Torch] {
        let mut world = flat_world(&[(0, 0)]);
        let mut falling = Falling::new();
        put(&mut world, &mut falling, at(FLOOR + 1), plant);
        assert_eq!(world.block(at(FLOOR + 1)), plant);
        put(&mut world, &mut falling, at(FLOOR + 2), Block::Gravel);
        let changed = falling.step(&mut world, |_| false);
        assert_eq!(world.block(at(FLOOR + 1)), Block::Gravel, "{plant:?}");
        assert!(changed.contains(&at(FLOOR + 1)));
    }
}

#[test]
fn plant_on_top_breaks_when_sand_leaves() {
    let mut world = flat_world(&[(0, 0)]);
    let mut falling = Falling::new();
    put(&mut world, &mut falling, at(15), Block::Sand);
    put(&mut world, &mut falling, at(16), Block::Poppy);
    falling.step(&mut world, |_| false);
    assert_eq!(world.block(at(14)), Block::Sand);
    assert_eq!(world.block(at(16)), Block::Air);
}

#[test]
fn waits_for_the_player_and_falls_after() {
    let mut world = flat_world(&[(0, 0)]);
    let mut falling = Falling::new();
    put(&mut world, &mut falling, at(FLOOR + 3), Block::Sand);
    let player = at(FLOOR + 2);
    for _ in 0..3 {
        let changed = falling.step(&mut world, |pos| pos == player);
        assert!(changed.is_empty());
        assert_eq!(world.block(at(FLOOR + 3)), Block::Sand);
        assert!(!falling.is_empty());
    }
    settle(&mut world, &mut falling);
    assert_eq!(world.block(at(FLOOR + 1)), Block::Sand);
}

#[test]
fn unloaded_columns_are_not_queued_or_touched() {
    let mut world = flat_world(&[(0, 0)]);
    let mut falling = Falling::new();
    falling.block_changed(&world, IVec3::new(15, 20, 4));
    assert_eq!(falling.len(), 6);
    falling.block_changed(&world, IVec3::new(16, 20, 4));
    assert_eq!(falling.len(), 6);
    assert!(falling.step(&mut world, |_| false).is_empty());
}

#[test]
fn placed_sand_in_the_air_falls() {
    let mut world = flat_world(&[(0, 0)]);
    let mut falling = Falling::new();
    put(&mut world, &mut falling, at(FLOOR + 5), Block::Sand);
    settle(&mut world, &mut falling);
    assert_eq!(world.block(at(FLOOR + 1)), Block::Sand);
}

#[test]
fn block_without_flag_stays() {
    let mut world = flat_world(&[(0, 0)]);
    let mut falling = Falling::new();
    put(&mut world, &mut falling, at(20), Block::Stone);
    assert!(falling.step(&mut world, |_| false).is_empty());
    assert_eq!(world.block(at(20)), Block::Stone);
}
