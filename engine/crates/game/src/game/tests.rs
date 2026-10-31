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
