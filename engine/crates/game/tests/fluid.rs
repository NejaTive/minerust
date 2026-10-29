//! Течение воды в `Game`: правка игрока запускает течение, а изменения от шагов
//! жидкости доходят до `interact` в том же кадре, что и правки игрока.
use std::collections::HashSet;

use game::Game;
use glam::IVec3;
use world::block::{Block, FLUID_SOURCE};
use world::column::Column;
use world::pos::{ColumnPos, SectionPos};
use world::world::affected_sections;

const FLOOR: i32 = 10;

fn game_with_floor() -> Game {
    let mut game = Game::new(1, None).unwrap();
    for z in -1..=1 {
        for x in -1..=1 {
            let mut column = Column::new();
            for lz in 0..16 {
                for lx in 0..16 {
                    column.set(IVec3::new(lx, FLOOR, lz), Block::Stone);
                }
            }
            let key = ColumnPos::new(x, z);
            game.world.insert(key, column);
            game.light.add_column(&game.world, key);
        }
    }
    game
}

/// Летим над полом и смотрим вниз.
fn look_down_at(game: &mut Game, x: f32, z: f32) {
    let pitch = -std::f32::consts::FRAC_PI_2;
    game.set_camera_state(&[x, 13.0, z, 0.0, pitch, 1.0])
        .unwrap();
}

#[test]
fn placed_water_flows_and_edits_reach_interact() {
    let mut game = game_with_floor();
    look_down_at(&mut game, 8.5, 8.5);
    assert!(game.select_block("water"));
    game.update(0.0);
    game.mouse_button(2, true);
    let first = game.interact(0.0);
    game.mouse_button(2, false);
    let source = IVec3::new(8, FLOOR + 1, 8);
    assert_eq!(game.world.block(source), Block::Water);
    assert_eq!(game.world.block_data(source), FLUID_SOURCE);
    assert!(first.dirty.contains(&SectionPos::of_block(source)));

    let mut dirty: HashSet<SectionPos> = HashSet::new();
    let mut time = 0.0;
    for _ in 0..40 {
        time += 0.1;
        game.update(time);
        dirty.extend(game.interact(time).dirty);
    }
    let reached = IVec3::new(8 + 7, FLOOR + 1, 8);
    assert_eq!(game.world.block(reached), Block::Water);
    assert_eq!(game.world.block(reached + IVec3::X), Block::Air);
    for section in affected_sections(reached) {
        assert!(dirty.contains(&section), "{section:?} was not remeshed");
    }
    assert!(game.world.drain_dirty().contains(&ColumnPos::new(0, 0)));
}

#[test]
fn paused_game_does_not_flow() {
    let mut game = game_with_floor();
    look_down_at(&mut game, 8.5, 8.5);
    game.select_block("water");
    game.update(0.0);
    game.mouse_button(2, true);
    game.interact(0.0);
    game.mouse_button(2, false);
    game.set_paused(true);
    let mut time = 0.0;
    for _ in 0..20 {
        time += 0.1;
        game.update(time);
        assert!(game.interact(time).dirty.is_empty());
    }
    assert_eq!(
        game.world.block(IVec3::new(9, FLOOR + 1, 8)),
        Block::Air,
        "water flowed on pause"
    );
}
