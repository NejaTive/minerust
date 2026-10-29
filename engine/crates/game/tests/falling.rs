use std::collections::HashSet;

use game::Game;
use glam::IVec3;
use world::block::Block;
use world::column::Column;
use world::pos::{ColumnPos, SectionPos};

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

fn aim_down_at(game: &mut Game, x: f32, z: f32) {
    let pitch = -std::f32::consts::FRAC_PI_2;
    game.set_camera_state(&[x, 15.0, z, 0.0, pitch, 1.0])
        .unwrap();
}

fn place(game: &mut Game, time: f32) {
    game.mouse_button(2, true);
    game.interact(time);
    game.mouse_button(2, false);
}

#[test]
fn sand_without_support_falls_and_reaches_interact() {
    let mut game = game_with_floor();
    game.select_block("sand");
    aim_down_at(&mut game, 8.5, 8.5);
    game.update(0.0);
    place(&mut game, 0.0);
    let sand = IVec3::new(8, FLOOR + 1, 8);
    assert_eq!(game.world.block(sand), Block::Sand);

    game.world.set_block(sand - IVec3::Y, Block::Air);
    game.select_block("stone");
    aim_down_at(&mut game, 8.5, 9.5);
    game.update(0.1);
    place(&mut game, 0.1);
    assert_eq!(game.world.block(IVec3::new(8, FLOOR + 1, 9)), Block::Stone);

    let mut dirty: HashSet<SectionPos> = HashSet::new();
    let mut time = 0.1;
    for _ in 0..40 {
        time += 0.1;
        game.update(time);
        dirty.extend(game.interact(time).dirty);
    }
    let bottom = IVec3::new(8, 0, 8);
    assert_eq!(game.world.block(bottom), Block::Sand);
    assert_eq!(game.world.block(sand), Block::Air);
    assert!(dirty.contains(&SectionPos::of_block(bottom)));
}

#[test]
fn paused_game_does_not_drop_sand() {
    let mut game = game_with_floor();
    game.select_block("sand");
    aim_down_at(&mut game, 8.5, 8.5);
    game.update(0.0);
    place(&mut game, 0.0);
    let sand = IVec3::new(8, FLOOR + 1, 8);
    game.world.set_block(sand - IVec3::Y, Block::Air);
    game.select_block("stone");
    aim_down_at(&mut game, 8.5, 9.5);
    place(&mut game, 0.1);
    game.set_paused(true);
    let mut time = 0.1;
    for _ in 0..20 {
        time += 0.1;
        game.update(time);
        game.interact(time);
    }
    assert_eq!(game.world.block(sand), Block::Sand);
}
