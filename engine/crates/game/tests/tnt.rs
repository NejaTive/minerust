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

fn tnt(x: i32, z: i32) -> IVec3 {
    IVec3::new(x, FLOOR + 1, z)
}

#[test]
fn right_click_primes_tnt_instead_of_placing_a_block() {
    let mut game = game_with_floor(Block::Stone);
    game.world.set_block(tnt(8, 8), Block::Tnt);
    game.select_block("stone");
    aim_down_at(&mut game, 8.5, 15.0, 8.5);
    game.update(0.0);
    right_click(&mut game, 0.0);
    assert_eq!(game.world.block(tnt(8, 8)), Block::Air);
    assert_eq!(game.world.block(tnt(8, 8) + IVec3::Y), Block::Air);
    assert_eq!(game.primed_tnt_count(), 1);
}

#[test]
fn primed_tnt_explodes_after_four_seconds_and_leaves_a_crater() {
    let mut game = game_with_floor(Block::Stone);
    game.world.set_block(tnt(8, 8), Block::Tnt);
    aim_down_at(&mut game, 8.5, 15.0, 8.5);
    game.update(0.0);
    right_click(&mut game, 0.0);
    let mut clock = Clock(0.0);
    clock.run(&mut game, 3.5);
    assert_eq!(game.primed_tnt_count(), 1);
    assert_eq!(game.world.block(IVec3::new(8, FLOOR, 8)), Block::Stone);
    clock.run(&mut game, 1.0);
    assert_eq!(game.primed_tnt_count(), 0);
    assert_eq!(game.world.block(IVec3::new(8, FLOOR, 8)), Block::Air);
    assert_eq!(game.world.block(IVec3::new(8, FLOOR - 4, 8)), Block::Stone);
}

#[test]
fn paused_game_keeps_the_fuse_still() {
    let mut game = game_with_floor(Block::Stone);
    game.world.set_block(tnt(8, 8), Block::Tnt);
    aim_down_at(&mut game, 8.5, 15.0, 8.5);
    game.update(0.0);
    right_click(&mut game, 0.0);
    game.set_paused(true);
    Clock(0.0).run(&mut game, 6.0);
    assert_eq!(game.primed_tnt_count(), 1);
    assert_eq!(game.world.block(IVec3::new(8, FLOOR, 8)), Block::Stone);
}

#[test]
fn obsidian_floor_survives_the_blast() {
    let mut game = game_with_floor(Block::Obsidian);
    game.world.set_block(tnt(8, 8), Block::Tnt);
    aim_down_at(&mut game, 8.5, 15.0, 8.5);
    game.update(0.0);
    right_click(&mut game, 0.0);
    Clock(0.0).run(&mut game, 5.0);
    assert_eq!(game.primed_tnt_count(), 0);
    assert_eq!(game.world.block(IVec3::new(8, FLOOR, 8)), Block::Obsidian);
}

#[test]
fn blast_chains_into_neighbouring_tnt() {
    let mut game = game_with_floor(Block::Stone);
    game.world.set_block(tnt(8, 8), Block::Tnt);
    game.world.set_block(tnt(9, 8), Block::Tnt);
    aim_down_at(&mut game, 8.5, 15.0, 8.5);
    game.update(0.0);
    right_click(&mut game, 0.0);
    let mut clock = Clock(0.0);
    clock.run(&mut game, 3.5);
    assert_eq!(game.world.block(tnt(9, 8)), Block::Tnt);
    clock.run(&mut game, 0.8);
    assert_eq!(game.world.block(tnt(9, 8)), Block::Air);
    assert_eq!(game.primed_tnt_count(), 1);
    clock.run(&mut game, 2.0);
    assert_eq!(game.primed_tnt_count(), 0);
}

#[test]
fn tnt_placed_next_to_a_powered_block_ignites() {
    let mut game = game_with_floor(Block::Stone);
    game.world
        .set_block(IVec3::new(8, FLOOR + 1, 8), Block::Stone);
    game.world.set_block_with_data(
        IVec3::new(8, FLOOR + 2, 8),
        Block::StonePressurePlate,
        world::block::PLATE_POWERED,
    );
    game.select_block("tnt");
    aim_down_at(&mut game, 9.5, 15.0, 8.5);
    game.update(0.0);
    right_click(&mut game, 0.0);
    Clock(0.0).run(&mut game, 0.3);
    assert_eq!(game.primed_tnt_count(), 1);
}

#[test]
fn standing_on_the_temple_plate_sets_off_the_trap() {
    let mut game = game_with_floor(Block::Stone);
    let trap = FLOOR + 1;
    for x in 7..=9 {
        for z in 7..=9 {
            game.world.set_block(IVec3::new(x, trap, z), Block::Tnt);
            game.world
                .set_block(IVec3::new(x, trap + 1, z), Block::Sandstone);
        }
    }
    let plate = IVec3::new(8, trap + 2, 8);
    game.world.set_block(plate, Block::StonePressurePlate);
    game.set_camera_state(&[8.5, plate.y as f32 + 1.62, 8.5, 0.0, 0.0, 0.0])
        .unwrap();
    let mut clock = Clock(0.0);
    clock.run(&mut game, 0.5);
    assert_eq!(game.primed_tnt_count(), 1);
    clock.run(&mut game, 3.8);
    assert_eq!(game.primed_tnt_count(), 8);
    clock.run(&mut game, 2.0);
    assert_eq!(game.primed_tnt_count(), 0);
    for x in 7..=9 {
        for z in 7..=9 {
            assert_eq!(game.world.block(IVec3::new(x, trap, z)), Block::Air);
        }
    }
}

#[test]
fn blast_pushes_the_player_away() {
    let mut game = game_with_floor(Block::Stone);
    game.world.set_block(tnt(8, 8), Block::Tnt);
    aim_down_at(&mut game, 8.5, 15.0, 8.5);
    game.update(0.0);
    right_click(&mut game, 0.0);
    game.set_camera_state(&[11.5, FLOOR as f32 + 1.0 + 1.62, 8.5, 0.0, 0.0, 0.0])
        .unwrap();
    let mut clock = Clock(0.0);
    clock.run(&mut game, 4.3);
    assert!(game.player().feet.x > 12.0, "{:?}", game.player().feet);
}

const LOADED_PLATE: IVec3 = IVec3::new(40, FLOOR + 1, 40);

fn game_with_loaded_pressed_plate() -> Game {
    let mut game = game_with_floor(Block::Stone);
    let mut column = Column::new();
    for lz in 0..16 {
        for lx in 0..16 {
            column.set(IVec3::new(lx, FLOOR, lz), Block::Stone);
        }
    }
    column.set_with_data(
        IVec3::new(8, FLOOR + 1, 8),
        Block::StonePressurePlate,
        world::block::PLATE_POWERED,
    );
    let key = ColumnPos::new(2, 2);
    game.world.insert(key, column);
    game.light.add_column(&game.world, key);
    game
}

#[test]
fn loaded_pressed_plate_lets_go_after_twenty_ticks() {
    let mut game = game_with_loaded_pressed_plate();
    let mut clock = Clock(0.0);
    clock.run(&mut game, 0.2);
    assert_eq!(
        game.world.block_data(LOADED_PLATE),
        world::block::PLATE_POWERED
    );
    clock.run(&mut game, 1.0);
    assert_eq!(game.world.block_data(LOADED_PLATE), 0);
}

#[test]
fn loaded_pressed_plate_stays_under_the_player() {
    let mut game = game_with_loaded_pressed_plate();
    game.set_camera_state(&[40.5, LOADED_PLATE.y as f32 + 1.62, 40.5, 0.0, 0.0, 0.0])
        .unwrap();
    Clock(0.0).run(&mut game, 2.0);
    assert_eq!(
        game.world.block_data(LOADED_PLATE),
        world::block::PLATE_POWERED
    );
}
