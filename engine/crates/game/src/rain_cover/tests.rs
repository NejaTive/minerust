use super::*;
use world::block::Block;
use world::column::Column;

fn flat_world(top: i32) -> World {
    let mut world = World::new();
    for x in -2..2 {
        for z in -2..2 {
            let mut column = Column::new();
            for lx in 0..SECTION_SIZE {
                for lz in 0..SECTION_SIZE {
                    for y in 0..=top {
                        column.set(IVec3::new(lx, y, lz), Block::Stone);
                    }
                }
            }
            world.insert(ColumnPos::new(x, z), column);
        }
    }
    world
}

#[test]
fn ground_sets_the_cover_height() {
    let world = flat_world(40);
    let mut cover = RainCover::new();
    cover.update(&world, 0, 0);
    for (x, z) in [(0, 0), (-16, -16), (15, 15), (3, -9)] {
        assert_eq!(cover.height(x, z), 41, "{x},{z}");
    }
}

#[test]
fn roof_above_ground_raises_the_cover() {
    let mut world = flat_world(40);
    world.set_block(IVec3::new(5, 60, 5), Block::Stone);
    let mut cover = RainCover::new();
    cover.update(&world, 0, 0);
    assert_eq!(cover.height(5, 5), 61);
    assert_eq!(cover.height(6, 5), 41);
}

#[test]
fn transparent_blocks_do_not_cover() {
    let mut world = flat_world(40);
    world.set_block(IVec3::new(2, 41, 2), Block::Leaves);
    world.set_block(IVec3::new(3, 41, 3), Block::Torch);
    let mut cover = RainCover::new();
    cover.update(&world, 0, 0);
    assert_eq!(cover.height(2, 2), 41);
    assert_eq!(cover.height(3, 3), 41);
}

#[test]
fn unloaded_columns_stay_open_and_resolve_when_loaded() {
    let mut world = World::new();
    let mut cover = RainCover::new();
    cover.update(&world, 0, 0);
    assert_eq!(cover.height(1, 1), 0);
    let mut column = Column::new();
    column.set(IVec3::new(1, 30, 1), Block::Stone);
    world.insert(ColumnPos::ZERO, column);
    cover.update(&world, 0, 0);
    assert_eq!(cover.height(1, 1), 31);
}

#[test]
fn edits_are_picked_up_after_invalidation() {
    let mut world = flat_world(40);
    let mut cover = RainCover::new();
    cover.update(&world, 0, 0);
    let roof = IVec3::new(4, 55, 4);
    world.set_block(roof, Block::Stone);
    cover.update(&world, 0, 0);
    assert_eq!(cover.height(4, 4), 41);
    cover.invalidate(roof);
    cover.update(&world, 0, 0);
    assert_eq!(cover.height(4, 4), 56);
    world.set_block(roof, Block::Air);
    cover.invalidate(roof);
    cover.update(&world, 0, 0);
    assert_eq!(cover.height(4, 4), 41);
}

#[test]
fn imported_column_is_recomputed() {
    let mut world = flat_world(40);
    let mut cover = RainCover::new();
    cover.update(&world, 0, 0);
    let mut column = Column::new();
    column.set(IVec3::new(0, 70, 0), Block::Stone);
    world.insert(ColumnPos::ZERO, column);
    cover.invalidate_column(ColumnPos::ZERO);
    cover.update(&world, 0, 0);
    assert_eq!(cover.height(0, 0), 71);
    assert_eq!(cover.height(1, 0), 0);
}

#[test]
fn window_follows_the_center() {
    let mut world = flat_world(40);
    world.set_block(IVec3::new(20, 50, 0), Block::Stone);
    let mut cover = RainCover::new();
    cover.update(&world, 0, 0);
    assert_ne!(cover.height(20, 0), 51);
    cover.update(&world, 10, 0);
    assert_eq!(cover.height(20, 0), 51);
    cover.update(&world, 40, 0);
    cover.update(&world, 10, 0);
    assert_eq!(cover.height(20, 0), 51);
}

#[test]
fn dirty_flag_reports_changes_once() {
    let world = flat_world(40);
    let mut cover = RainCover::new();
    cover.update(&world, 0, 0);
    assert!(cover.take_dirty().is_some());
    cover.update(&world, 0, 0);
    assert!(cover.take_dirty().is_none());
}
