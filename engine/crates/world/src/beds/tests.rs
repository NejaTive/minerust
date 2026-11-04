use super::*;
use crate::bed;
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

#[test]
fn a_bed_is_placed_as_foot_and_head() {
    let mut world = world();
    let foot = at(4, 4);
    let changed = world.place_bed(foot, IVec3::Z, 14);
    assert_eq!(changed, vec![foot, foot + IVec3::Z]);
    assert_eq!(world.block(foot), Block::Bed);
    assert_eq!(world.block(foot + IVec3::Z), Block::Bed);
    assert!(!bed::is_head(world.block_data(foot)));
    assert!(bed::is_head(world.block_data(foot + IVec3::Z)));
    assert_eq!(bed::color_of(world.block_data(foot)), 14);
    assert_eq!(
        bed::toward_head(world.block_data(foot + IVec3::Z)),
        IVec3::Z
    );
}

#[test]
fn a_bed_needs_a_floor_and_room_for_both_halves() {
    let mut world = world();
    assert!(world.can_place_bed(at(4, 4), IVec3::X));
    world.set_block(at(5, 4), Block::Stone);
    assert!(!world.can_place_bed(at(4, 4), IVec3::X));
    world.set_block(at(5, 4) + IVec3::NEG_Y, Block::Air);
    world.set_block(at(5, 4), Block::Air);
    assert!(!world.can_place_bed(at(4, 4), IVec3::X));
    assert!(world.place_bed(at(4, 4), IVec3::X, 0).is_empty());
}

#[test]
fn breaking_either_half_removes_the_whole_bed() {
    for broken_head in [false, true] {
        let mut world = world();
        let foot = at(4, 4);
        world.place_bed(foot, IVec3::NEG_X, 3);
        let broken = if broken_head {
            foot + IVec3::NEG_X
        } else {
            foot
        };
        let changed = world.set_block_supported(broken, Block::Air);
        assert_eq!(changed.len(), 2);
        assert_eq!(world.block(foot), Block::Air);
        assert_eq!(world.block(foot + IVec3::NEG_X), Block::Air);
    }
}
