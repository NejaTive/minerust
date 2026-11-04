use glam::IVec2;

use super::*;
use crate::weather::TRANSITION_SECONDS;

const SEED: i32 = 21;

fn spot(terrain: &Terrain, desert: bool) -> (i32, i32) {
    (-2000..2000)
        .step_by(37)
        .flat_map(|x| (-2000..2000).step_by(37).map(move |z| (x, z)))
        .find(|&(x, z)| (terrain.biome_at(x, z) == Biome::Desert) == desert)
        .expect("biome not found near origin")
}

fn game_at(spot: (i32, i32)) -> Game {
    let mut game = Game::new(SEED, None).unwrap();
    game.set_camera_state(&[spot.0 as f32, 80.0, spot.1 as f32, 0.0, 0.0, 1.0])
        .unwrap();
    game
}

fn run(game: &mut Game, seconds: f32) {
    let frames = (seconds / MAX_DT) as u32;
    for frame in 1..=frames {
        game.update(frame as f32 * MAX_DT);
    }
}

fn remaining(game: &Game) -> u32 {
    game.weather_state()[1] as u32
}

#[test]
fn weather_ticks_with_the_game_clock() {
    let mut game = Game::new(SEED, None).unwrap();
    let start = remaining(&game);
    run(&mut game, 10.0);
    let spent = start - remaining(&game);
    assert!((500..=605).contains(&spent), "{spent}");
}

#[test]
fn time_acceleration_speeds_up_weather() {
    let mut slow = Game::new(SEED, None).unwrap();
    let mut fast = Game::new(SEED, None).unwrap();
    fast.set_time_fast(true);
    let start = remaining(&slow);
    run(&mut slow, 5.0);
    run(&mut fast, 5.0);
    let spent = |game: &Game| start - remaining(game);
    assert!(spent(&slow) > 200);
    assert_eq!(spent(&fast), 60 * spent(&slow));
}

#[test]
fn paused_and_frozen_weather_stands_still() {
    let mut game = Game::new(SEED, None).unwrap();
    game.cycle_weather();
    let before = game.weather_state();
    game.set_paused(true);
    run(&mut game, 5.0);
    assert_eq!(game.weather_state(), before);
    game.set_paused(false);
    game.set_time_frozen(true);
    game.set_time_fast(true);
    run(&mut game, 5.0);
    assert_eq!(remaining(&game), before[1] as u32);
}

#[test]
fn cycling_changes_kind_and_rain_follows_smoothly() {
    let mut game = Game::new(SEED, None).unwrap();
    assert_eq!(game.weather(), Weather::Clear);
    game.cycle_weather();
    assert_eq!(game.weather(), Weather::Rain);
    run(&mut game, TRANSITION_SECONDS / 2.0);
    assert!((0.3..0.6).contains(&game.rain()), "{}", game.rain());
    run(&mut game, TRANSITION_SECONDS);
    assert_eq!(game.rain(), 1.0);
    game.cycle_weather();
    assert_eq!(game.weather(), Weather::Thunder);
    game.cycle_weather();
    assert_eq!(game.weather(), Weather::Clear);
}

#[test]
fn disabled_weather_is_always_clear() {
    let mut game = Game::new(SEED, None).unwrap();
    game.cycle_weather();
    run(&mut game, 20.0);
    game.set_weather_enabled(false);
    run(&mut game, 20.0);
    assert_eq!(game.rain(), 0.0);
    game.cycle_weather();
    run(&mut game, 20.0);
    assert_eq!((game.weather(), game.rain()), (Weather::Rain, 0.0));
    game.set_time_fast(true);
    run(&mut game, 60.0);
    assert_eq!(game.rain(), 0.0);
}

#[test]
fn desert_has_no_drops_but_a_grey_sky() {
    let terrain = Terrain::new(SEED);
    let mut desert = game_at(spot(&terrain, true));
    let mut meadow = game_at(spot(&terrain, false));
    let dry_sky = desert.sky().color;
    for game in [&mut desert, &mut meadow] {
        game.cycle_weather();
        run(game, 20.0);
    }
    assert_eq!(desert.rain(), 1.0);
    assert_eq!(desert.rain_drops(), 0.0);
    assert!(desert.sky().color != dry_sky);
    assert_eq!(meadow.rain_drops(), 1.0);
}

#[test]
fn weather_state_survives_save_and_load() {
    let mut game = Game::new(SEED, None).unwrap();
    game.cycle_weather();
    game.cycle_weather();
    run(&mut game, 20.0);
    let saved = game.weather_state();
    let mut loaded = Game::new(SEED, None).unwrap();
    loaded.set_weather_state(&saved);
    assert_eq!(loaded.weather(), Weather::Thunder);
    assert_eq!(loaded.rain(), game.rain());
    assert_eq!(loaded.weather_state(), saved);
    loaded.set_weather_state(&[1.0]);
    assert_eq!(loaded.weather_state(), saved);
}

#[test]
fn rain_cover_sees_roofs_after_edits() {
    let terrain = Terrain::new(SEED);
    let (x, z) = spot(&terrain, false);
    let mut game = game_at((x, z));
    let key = game.camera_column();
    for dx in -3..=3 {
        for dz in -3..=3 {
            game.world.insert(key + IVec2::new(dx, dz), Column::new());
        }
    }
    game.cycle_weather();
    run(&mut game, 20.0);
    assert!(game.take_rain_cover().is_some());
    let roof = IVec3::new(x + 2, 70, z + 2);
    game.world.set_block(roof, Block::Stone);
    game.apply_pending(&[roof]);
    run(&mut game, 0.2);
    assert!(game.take_rain_cover().is_some());
    assert_eq!(game.rain_cover.height(roof.x, roof.z), 71);
}

fn load_around_origin(game: &mut Game) {
    for x in -1..=1 {
        for z in -1..=1 {
            let key = ColumnPos::new(x, z);
            let mut column = game.terrain.generate(key);
            column.populated = true;
            game.world.insert(key, column);
        }
    }
}

fn pig_above_ground(game: &Game) -> Mob {
    let ground = (0..WORLD_HEIGHT)
        .rev()
        .find(|&y| game.world.block(IVec3::new(5, y, 5)) != Block::Air)
        .unwrap();
    Mob::new(
        MobKind::Pig,
        Vec3::new(5.5, ground as f32 + 1.0, 5.5),
        0.0,
        3,
    )
}

#[test]
fn mobs_are_saved_with_their_column_and_return_with_it() {
    let mut game = Game::new(SEED, None).unwrap();
    load_around_origin(&mut game);
    game.mobs.push(pig_above_ground(&game));
    run(&mut game, 2.0);
    let key = ColumnPos::new(0, 0);
    assert_eq!(game.world.edited_column(key).unwrap().mobs.len(), 1);
    assert_eq!(game.mob_count(), 1);

    let saved = world::save::encode_column(game.world.edited_column(key).unwrap());
    game.world.remove(key);
    run(&mut game, 0.5);
    assert_eq!(game.mob_count(), 0);

    let restored = world::save::decode_column(&saved).unwrap();
    game.world.insert(key, restored);
    run(&mut game, 0.5);
    assert_eq!(game.mob_count(), 1);
}

#[test]
fn unloaded_columns_freeze_and_drop_their_mobs() {
    let mut game = Game::new(SEED, None).unwrap();
    load_around_origin(&mut game);
    game.mobs.push(pig_above_ground(&game));
    run(&mut game, 1.0);
    game.world.remove(ColumnPos::new(0, 0));
    run(&mut game, 1.0);
    assert!(game.mobs().is_empty());
}

#[test]
fn spawn_egg_summons_a_pig_without_being_spent() {
    let mut game = Game::new(SEED, None).unwrap();
    load_around_origin(&mut game);
    assert!(game.select_block("pig_spawn_egg"));
    let ground = (0..WORLD_HEIGHT)
        .rev()
        .find(|&y| game.world.block(IVec3::new(5, y, 5)) != Block::Air)
        .unwrap();
    let hit = Hit {
        block: IVec3::new(5, ground, 5),
        normal: IVec3::Y,
        distance: 1.0,
        bounds: None,
    };
    game.use_item(Item::PigSpawnEgg, hit);
    assert_eq!(game.mob_count(), 1);
    assert_eq!(game.selected, Some(Held::Item(Item::PigSpawnEgg)));
    assert!(!game.select_block("nope"));
}

const FLOOR: i32 = 10;

fn floor_game() -> Game {
    let mut game = Game::new(1, None).unwrap();
    for x in -1..=1 {
        for z in -1..=1 {
            let mut column = Column::new();
            for lx in 0..16 {
                for lz in 0..16 {
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

fn pig_at(x: f32, z: f32, seed: u32) -> Mob {
    Mob::new(MobKind::Pig, Vec3::new(x, FLOOR as f32 + 1.0, z), 0.0, seed)
}

fn mob_distance(game: &Game) -> f32 {
    game.mobs[0].body.feet.distance(game.mobs[1].body.feet)
}

#[test]
fn two_pigs_in_one_spot_drift_apart() {
    let mut game = floor_game();
    game.mobs.push(pig_at(8.5, 8.5, 1));
    game.mobs.push(pig_at(8.5, 8.5, 2));
    game.set_camera_state(&[8.5, 20.0, 30.0, 0.0, 0.0, 1.0])
        .unwrap();
    run(&mut game, 1.0);
    assert!(mob_distance(&game) > 0.85, "{}", mob_distance(&game));
}

#[test]
fn overlapping_pigs_are_pushed_gently() {
    let mut game = floor_game();
    game.mobs.push(pig_at(8.5, 8.5, 1));
    game.mobs.push(pig_at(8.9, 8.5, 2));
    game.set_camera_state(&[8.5, 20.0, 30.0, 0.0, 0.0, 1.0])
        .unwrap();
    let before = mob_distance(&game);
    game.update(0.0);
    game.update(TICK);
    let after = mob_distance(&game);
    assert!(after > before && after - before < 0.2);
}

fn walk_forward(game: &mut Game, seconds: f32) -> f32 {
    let start = game.player().feet.x;
    game.set_key("KeyW", true);
    run(game, seconds);
    game.player().feet.x - start
}

#[test]
fn walking_into_a_pig_pushes_it_and_slows_the_player() {
    let state = [4.5, FLOOR as f32 + 1.0 + EYE_HEIGHT, 8.5, 0.0, 0.0, 0.0];
    let mut free = floor_game();
    free.set_camera_state(&state).unwrap();
    let unobstructed = walk_forward(&mut free, 1.0);

    let mut game = floor_game();
    game.set_camera_state(&state).unwrap();
    game.mobs.push(pig_at(7.0, 8.5, 1));
    let start = game.mobs[0].body.feet.x;
    let obstructed = walk_forward(&mut game, 1.0);
    assert!(obstructed < unobstructed, "{obstructed} {unobstructed}");
    assert!(obstructed > 0.0);
    assert!(game.mobs[0].body.feet.x > start + 0.3);
}

fn try_place(game: &mut Game, name: &str) -> Block {
    game.set_camera_state(&[8.5, 14.5, 8.5, 0.0, -std::f32::consts::FRAC_PI_2, 1.0])
        .unwrap();
    assert!(game.select_block(name));
    game.update(0.0);
    game.mouse_button(2, true);
    game.interact(0.0);
    game.mouse_button(2, false);
    game.world.block(IVec3::new(8, FLOOR + 1, 8))
}

#[test]
fn solid_blocks_are_not_placed_into_a_pig_but_flowers_are() {
    let mut game = floor_game();
    game.mobs.push(pig_at(8.5, 8.5, 1));
    assert_eq!(try_place(&mut game, "stone"), Block::Air);
    assert_eq!(try_place(&mut game, "poppy"), Block::Poppy);

    let mut empty = floor_game();
    assert_eq!(try_place(&mut empty, "stone"), Block::Stone);
}

#[test]
fn generated_springs_start_flowing_after_the_column_is_inserted() {
    let terrain = Terrain::new(SEED);
    let key = (-12..12)
        .flat_map(|z| (-12..12).map(move |x| ColumnPos::new(x, z)))
        .find(|&key| !terrain.generate(key).springs.is_empty())
        .expect("a spring column near the origin");
    let mut game = game_at((key.origin().x, key.origin().z));
    let column = game.terrain.generate(key);
    let spring = key.origin() + column.springs[0];
    game.world.insert(key, column);
    run(&mut game, 0.5);
    assert!(game.world.drain_fluid_wakes().is_empty());
    let cells = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z, IVec3::NEG_Y]
        .map(|d| game.world.block(spring + d));
    assert!(
        cells.iter().any(|b| b.fluid().is_some()),
        "no liquid beside {spring}: {cells:?}"
    );
}

fn fall_height_after(mut game: Game, seconds: f32) -> f32 {
    game.mobs.push(Mob::new(
        MobKind::Pig,
        Vec3::new(8.5, FLOOR as f32 + 8.0, 8.5),
        0.0,
        1,
    ));
    game.set_camera_state(&[8.5, FLOOR as f32 + 9.6, 12.5, 0.0, 0.0, 0.0])
        .unwrap();
    run(&mut game, seconds);
    game.mobs[0].body.feet.y
}

#[test]
fn a_web_holds_a_falling_mob_but_air_does_not() {
    let free = fall_height_after(floor_game(), 0.5);
    let mut webbed = floor_game();
    for y in FLOOR + 1..FLOOR + 12 {
        webbed.world.set_block(IVec3::new(8, y, 8), Block::Web);
    }
    let held = fall_height_after(webbed, 0.5);
    assert!(free < FLOOR as f32 + 7.0, "{free}");
    assert!(held > FLOOR as f32 + 7.8, "{held}");
}

#[test]
fn a_web_holds_a_falling_player() {
    let mut game = floor_game();
    for y in FLOOR + 1..FLOOR + 12 {
        game.world.set_block(IVec3::new(8, y, 8), Block::Web);
    }
    game.set_camera_state(&[8.5, FLOOR as f32 + 8.0 + EYE_HEIGHT, 8.5, 0.0, 0.0, 0.0])
        .unwrap();
    run(&mut game, 0.5);
    assert!(
        game.player().feet.y > FLOOR as f32 + 7.8,
        "{}",
        game.player().feet.y
    );
}

fn right_click_down_on(game: &mut Game, name: &str) {
    game.set_camera_state(&[8.5, 14.5, 8.5, 0.0, -std::f32::consts::FRAC_PI_2, 1.0])
        .unwrap();
    assert!(game.select_block(name));
    game.update(0.0);
    game.mouse_button(2, true);
    game.interact(0.0);
    game.mouse_button(2, false);
}

const SPOT: IVec3 = IVec3::new(8, FLOOR + 1, 8);

#[test]
fn right_click_on_a_chest_opens_its_window_and_places_nothing() {
    let mut game = floor_game();
    game.world.set_block_with_data(SPOT, Block::Chest, 0);
    right_click_down_on(&mut game, "stone");
    assert!(game.take_container_opened());
    assert!(!game.take_container_opened());
    assert_eq!(game.container_slots(), vec![None; 27]);
    assert_eq!(game.world.block(SPOT + IVec3::Y), Block::Air);
    assert!(game.set_container_slot(4, Some("dirt".into())));
    assert!(!game.set_container_slot(5, Some("no_such_block".into())));
    assert_eq!(game.container_slots()[4].as_deref(), Some("dirt"));
    game.close_container();
    assert!(game.container_slots().is_empty());
}

#[test]
fn a_chest_under_a_solid_block_stays_shut() {
    let mut game = floor_game();
    game.world.set_block_with_data(SPOT, Block::Chest, 0);
    game.world.set_block(SPOT + IVec3::Y, Block::Stone);
    game.set_camera_state(&[8.5, 14.5, 8.5, 0.0, -std::f32::consts::FRAC_PI_2, 1.0])
        .unwrap();
    game.update(0.0);
    assert!(game.world.chest_blocked(SPOT));
    game.mouse_button(2, true);
    game.interact(0.0);
    assert!(!game.take_container_opened());
}

#[test]
fn a_flower_in_hand_goes_into_the_pot_once() {
    let mut game = floor_game();
    game.world.set_block(SPOT, Block::FlowerPot);
    right_click_down_on(&mut game, "poppy");
    assert_eq!(
        Block::pot_plant(game.world.block_data(SPOT)),
        Some(Block::Poppy)
    );
    right_click_down_on(&mut game, "dandelion");
    assert_eq!(
        Block::pot_plant(game.world.block_data(SPOT)),
        Some(Block::Poppy)
    );
    assert_eq!(game.world.block(SPOT + IVec3::Y), Block::Air);
}

#[test]
fn a_pot_only_takes_the_click_when_the_hand_holds_a_plant() {
    let mut game = floor_game();
    game.world.set_block(SPOT, Block::FlowerPot);
    assert!(game.select_block("stone"));
    assert!(!game.pot_accepts(SPOT));
    assert!(game.select_block("red_mushroom"));
    assert!(game.pot_accepts(SPOT));
    game.world.plant_in_pot(SPOT, Block::RedMushroom);
    assert!(!game.pot_accepts(SPOT));
}
