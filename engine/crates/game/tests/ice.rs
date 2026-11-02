use std::collections::HashSet;

use game::Game;
use glam::IVec3;
use world::block::{Block, FLUID_SOURCE};
use world::column::Column;
use world::pos::{ColumnPos, SectionPos};
use world::world::affected_sections;

const FLOOR: i32 = 10;
const ICE_LEVEL: i32 = FLOOR + 1;

fn frozen_pond() -> Game {
    let mut game = Game::new(1, None).unwrap();
    for z in -1..=1 {
        for x in -1..=1 {
            let mut column = Column::new();
            for lz in 0..16 {
                for lx in 0..16 {
                    column.set(IVec3::new(lx, FLOOR, lz), Block::Stone);
                    column.set(IVec3::new(lx, ICE_LEVEL, lz), Block::Ice);
                }
            }
            let key = ColumnPos::new(x, z);
            game.world.insert(key, column);
            game.light.add_column(&game.world, key);
        }
    }
    let pitch = -std::f32::consts::FRAC_PI_2;
    game.set_camera_state(&[8.5, 14.0, 8.5, 0.0, pitch, 1.0])
        .unwrap();
    game
}

fn break_target(game: &mut Game) -> HashSet<SectionPos> {
    game.update(0.0);
    game.mouse_button(0, true);
    let edits = game.interact(0.0);
    game.mouse_button(0, false);
    edits.dirty.into_iter().collect()
}

#[test]
fn breaking_ice_leaves_a_water_source() {
    let mut game = frozen_pond();
    let pos = IVec3::new(8, ICE_LEVEL, 8);
    let dirty = break_target(&mut game);
    assert_eq!(game.world.block(pos), Block::Water);
    assert_eq!(game.world.block_data(pos), FLUID_SOURCE);
    for section in affected_sections(pos) {
        assert!(dirty.contains(&section), "{section:?} was not remeshed");
    }
}

#[test]
fn water_from_broken_ice_flows_into_the_hole_neighbours() {
    let mut game = frozen_pond();
    for x in 7..=9 {
        for z in 7..=9 {
            if (x, z) != (8, 8) {
                game.world
                    .set_block(IVec3::new(x, ICE_LEVEL, z), Block::Air);
            }
        }
    }
    break_target(&mut game);
    let mut time = 0.0;
    for _ in 0..40 {
        time += 0.1;
        game.update(time);
        game.interact(time);
    }
    assert_eq!(game.world.block(IVec3::new(9, ICE_LEVEL, 8)), Block::Water);
    assert_eq!(game.world.block(IVec3::new(8, ICE_LEVEL, 9)), Block::Water);
}

#[test]
fn other_blocks_still_break_to_air() {
    let mut game = frozen_pond();
    game.world
        .set_block(IVec3::new(8, ICE_LEVEL, 8), Block::Stone);
    break_target(&mut game);
    assert_eq!(game.world.block(IVec3::new(8, ICE_LEVEL, 8)), Block::Air);
}
