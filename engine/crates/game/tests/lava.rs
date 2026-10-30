use game::Game;
use glam::IVec3;
use world::block::{Block, FLUID_SOURCE, FluidKind};
use world::column::Column;
use world::pos::ColumnPos;

const FLOOR: i32 = 10;
const Y: i32 = FLOOR + 1;

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

fn place(game: &mut Game, block: &str, x: i32, z: i32, time: &mut f32) {
    place_at(game, block, IVec3::new(x, Y, z), time);
}

fn place_at(game: &mut Game, block: &str, pos: IVec3, time: &mut f32) {
    let (x, z) = (pos.x, pos.z);
    let pitch = -std::f32::consts::FRAC_PI_2;
    game.set_camera_state(&[x as f32 + 0.5, 13.0, z as f32 + 0.5, 0.0, pitch, 1.0])
        .unwrap();
    assert!(game.select_block(block));
    *time += 0.3;
    game.update(*time);
    game.mouse_button(2, true);
    game.interact(*time);
    game.mouse_button(2, false);
    assert_eq!(game.world.block(pos).def().name, block);
}

fn run(game: &mut Game, seconds: f32, time: &mut f32) {
    for _ in 0..(seconds * 10.0) as usize {
        *time += 0.1;
        game.update(*time);
        game.interact(*time);
    }
}

fn reach(game: &Game, block: Block, x: i32, z: i32) -> i32 {
    (1..16)
        .take_while(|&d| game.world.block(IVec3::new(x + d, Y, z)) == block)
        .count() as i32
}

#[test]
fn lava_flows_slower_than_water_and_stops_after_three_cells() {
    let mut game = game_with_floor();
    let mut time = 0.0;
    place(&mut game, "lava", 4, 4, &mut time);
    place(&mut game, "water", 4, 20, &mut time);
    run(&mut game, 1.2, &mut time);
    let (lava, water) = (
        reach(&game, Block::Lava, 4, 4),
        reach(&game, Block::Water, 4, 20),
    );
    assert!(water > lava && lava >= 1, "lava {lava}, water {water}");
    run(&mut game, 4.0, &mut time);
    assert_eq!(reach(&game, Block::Lava, 4, 4), 3);
    assert_eq!(reach(&game, Block::Water, 4, 20), 7);
    assert_eq!(
        game.world.block_data(IVec3::new(4, Y, 4)),
        FLUID_SOURCE,
        "source stays"
    );
}

#[test]
fn water_meeting_lava_makes_obsidian_and_cobblestone() {
    let mut game = game_with_floor();
    let mut time = 0.0;
    place(&mut game, "lava", 4, 4, &mut time);
    run(&mut game, 4.0, &mut time);
    place(&mut game, "water", 7, 5, &mut time);
    run(&mut game, 3.0, &mut time);
    assert_eq!(game.world.block(IVec3::new(7, Y, 4)), Block::Cobblestone);
    assert_eq!(game.world.block(IVec3::new(4, Y, 4)), Block::Lava);

    place(&mut game, "water", 4, 5, &mut time);
    run(&mut game, 3.0, &mut time);
    assert_eq!(game.world.block(IVec3::new(4, Y, 4)), Block::Obsidian);
}

#[test]
fn eye_fluid_reports_lava_around_the_camera() {
    let mut game = game_with_floor();
    let mut time = 0.0;
    place(&mut game, "lava", 4, 4, &mut time);
    assert_eq!(game.eye_fluid(), None);
    game.set_camera_state(&[4.5, Y as f32 + 0.3, 4.5, 0.0, 0.0, 1.0])
        .unwrap();
    assert_eq!(game.eye_fluid(), Some(FluidKind::Lava));
}

fn aim_down(game: &mut Game, x: f32, z: f32) {
    let pitch = -std::f32::consts::FRAC_PI_2;
    game.set_camera_state(&[x, 15.0, z, 0.0, pitch, 1.0])
        .unwrap();
}

fn click_place(game: &mut Game, name: &str, time: &mut f32) {
    assert!(game.select_block(name));
    *time += 0.3;
    game.update(*time);
    game.mouse_button(2, true);
    game.interact(*time);
    game.mouse_button(2, false);
}

#[test]
fn fluid_in_hand_refills_a_flowing_cell_of_the_same_fluid() {
    for (name, block) in [("water", Block::Water), ("lava", Block::Lava)] {
        let mut game = game_with_floor();
        let flowing = IVec3::new(5, Y, 5);
        game.world.set_block_with_data(flowing, block, 3);
        let mut time = 0.0;
        aim_down(&mut game, 5.5, 5.5);
        click_place(&mut game, name, &mut time);
        assert_eq!(game.world.block(flowing), block, "{name}");
        assert_eq!(game.world.block_data(flowing), FLUID_SOURCE, "{name}");
        assert_eq!(game.world.block(flowing + IVec3::Y), Block::Air, "{name}");
    }
}

#[test]
fn other_blocks_and_other_fluids_go_through_a_flowing_cell_to_the_floor() {
    let flowing = IVec3::new(5, Y, 5);
    for (name, block) in [("stone", Block::Stone), ("lava", Block::Lava)] {
        let mut game = game_with_floor();
        game.world.set_block_with_data(flowing, Block::Water, 3);
        let mut time = 0.0;
        aim_down(&mut game, 5.5, 5.5);
        click_place(&mut game, name, &mut time);
        assert_eq!(game.world.block(flowing), block, "{name}");
        assert_eq!(game.world.block(flowing + IVec3::Y), Block::Air, "{name}");
    }
}

#[test]
fn fluid_in_hand_ignores_a_source_of_the_same_fluid() {
    let mut game = game_with_floor();
    let source = IVec3::new(5, Y, 5);
    game.world.set_block(source, Block::Water);
    let mut time = 0.0;
    aim_down(&mut game, 5.5, 5.5);
    click_place(&mut game, "water", &mut time);
    assert_eq!(game.world.block(source), Block::Water);
    assert_eq!(game.world.block_data(source), FLUID_SOURCE);
    assert_eq!(game.world.block(source + IVec3::Y), Block::Air);
}
