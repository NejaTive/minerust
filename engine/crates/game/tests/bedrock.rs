use game::Game;
use glam::IVec3;
use world::block::Block;
use world::pos::ColumnPos;
use world::terrain::{FLAT_SURFACE, WorldType};

fn aim_down(game: &mut Game, height: f32) {
    let pitch = -std::f32::consts::FRAC_PI_2;
    game.set_camera_state(&[0.5, height, 0.5, 0.0, pitch, 1.0])
        .unwrap();
}

fn flat_game() -> Game {
    let mut game = Game::of(1, WorldType::Flat, None).unwrap();
    let key = ColumnPos::ZERO;
    game.world.insert(key, game.terrain.generate(key));
    game.light.add_column(&game.world, key);
    game
}

#[test]
fn flat_spawn_stands_on_the_surface() {
    let game = Game::of(1, WorldType::Flat, None).unwrap();
    assert_eq!(game.terrain.world_type(), WorldType::Flat);
    assert!((game.camera_state()[1] - (FLAT_SURFACE as f32 + 1.0)).abs() < 3.0);
}

#[test]
fn bedrock_cannot_be_broken() {
    let mut game = flat_game();
    for y in 1..=FLAT_SURFACE {
        game.world
            .set_block_supported(IVec3::new(0, y, 0), Block::Air);
    }
    aim_down(&mut game, 3.5);
    game.mouse_button(0, true);
    game.interact(0.0);
    game.interact(1.0);
    assert_eq!(game.world.block(IVec3::new(0, 1, 0)), Block::Air);
    assert_eq!(game.world.block(IVec3::new(0, 0, 0)), Block::Bedrock);
}

#[test]
fn stone_above_bedrock_still_breaks() {
    let mut game = flat_game();
    aim_down(&mut game, 8.0);
    game.mouse_button(0, true);
    game.interact(0.0);
    assert_eq!(game.world.block(IVec3::new(0, FLAT_SURFACE, 0)), Block::Air);
}
