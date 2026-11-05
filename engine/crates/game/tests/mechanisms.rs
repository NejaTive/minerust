use game::Game;
use glam::IVec3;
use world::block::Block;
use world::column::Column;
use world::pos::ColumnPos;

const FLOOR: i32 = 10;

fn game_with_floor(floor: Block) -> Game {
    let mut game = Game::new(1, None).unwrap();
    for z in -1..=1 {
        for x in -1..=1 {
            let mut column = Column::new();
            for lz in 0..16 {
                for lx in 0..16 {
                    for y in 0..=FLOOR {
                        column.set(IVec3::new(lx, y, lz), floor);
                    }
                }
            }
            let key = ColumnPos::new(x, z);
            game.world.insert(key, column);
            game.light.add_column(&game.world, key);
        }
    }
    game
}

fn aim_down_at(game: &mut Game, x: f32, y: f32, z: f32) {
    let pitch = -std::f32::consts::FRAC_PI_2;
    game.set_camera_state(&[x, y, z, 0.0, pitch, 1.0]).unwrap();
}

fn right_click(game: &mut Game, time: f32) {
    game.mouse_button(2, true);
    game.interact(time);
    game.mouse_button(2, false);
}

struct Clock(f32);

impl Clock {
    fn run(&mut self, game: &mut Game, seconds: f32) {
        for _ in 0..(seconds / 0.1).round() as u32 {
            self.0 += 0.1;
            game.update(self.0);
            game.interact(self.0);
        }
    }
}

#[test]
fn right_click_on_a_lever_next_to_tnt_sets_it_off() {
    let mut game = game_with_floor(Block::Stone);
    let lever = IVec3::new(8, FLOOR + 1, 8);
    game.world
        .set_block_with_data(lever, Block::Lever, world::block::LEVER_FLOOR);
    game.world
        .set_block(IVec3::new(9, FLOOR + 1, 8), Block::Tnt);
    aim_down_at(&mut game, 8.5, 15.0, 8.5);
    game.update(0.0);
    let mut clock = Clock(0.0);
    clock.run(&mut game, 0.3);
    assert_eq!(game.primed_tnt_count(), 0);
    right_click(&mut game, 0.4);
    clock.run(&mut game, 0.3);
    assert_ne!(
        game.world.block_data(lever) & world::block::LEVER_POWERED,
        0
    );
    assert_eq!(game.primed_tnt_count(), 1);
}

#[test]
fn walking_into_a_tripwire_powers_the_hooks_and_lets_go_again() {
    use world::block::{FACING_NEG_X, FACING_POS_X, HOOK_POWERED};
    let mut game = game_with_floor(Block::Stone);
    let y = FLOOR + 1;
    let (west, east) = (IVec3::new(5, y, 8), IVec3::new(11, y, 8));
    for x in [4, 12] {
        game.world.set_block(IVec3::new(x, y, 8), Block::Stone);
    }
    game.world
        .set_block_with_data(west, Block::TripwireHook, FACING_POS_X);
    game.world
        .set_block_with_data(east, Block::TripwireHook, FACING_NEG_X);
    for x in 6..=10 {
        game.world.set_block(IVec3::new(x, y, 8), Block::Tripwire);
    }
    game.world.set_block(west + IVec3::Y, Block::Tnt);
    let mut clock = Clock(0.0);
    game.set_camera_state(&[2.5, y as f32 + 1.62, 2.5, 0.0, 0.0, 0.0])
        .unwrap();
    clock.run(&mut game, 0.5);
    assert_eq!(game.primed_tnt_count(), 0);
    game.set_camera_state(&[8.5, y as f32 + 1.62, 8.5, 0.0, 0.0, 0.0])
        .unwrap();
    clock.run(&mut game, 0.3);
    assert_ne!(game.world.block_data(west) & HOOK_POWERED, 0);
    assert_ne!(game.world.block_data(east) & HOOK_POWERED, 0);
    assert_eq!(game.primed_tnt_count(), 1);
    game.set_camera_state(&[2.5, y as f32 + 1.62, 2.5, 0.0, 0.0, 0.0])
        .unwrap();
    clock.run(&mut game, 1.0);
    assert_eq!(game.world.block_data(west) & HOOK_POWERED, 0);
}
