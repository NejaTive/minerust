use game::Game;
use game::weather::Weather;
use world::terrain::Biome;

const SEED: i32 = 1337;

fn find(biome: Biome) -> (i32, i32) {
    let game = Game::new(SEED, None).unwrap();
    (-400..400)
        .flat_map(|z| (-400..400).map(move |x| (x * 16, z * 16)))
        .find(|&(x, z)| (0..8).all(|d| game.terrain.biome_at(x + d * 5, z + d * 5) == biome))
        .unwrap_or_else(|| panic!("no {biome:?}"))
}

fn game_at(biome: Biome) -> Game {
    let (x, z) = find(biome);
    let mut game = Game::new(
        SEED,
        Some(&[x as f32 + 0.5, 150.0, z as f32 + 0.5, 0.0, 0.0, 1.0]),
    )
    .unwrap();
    game.set_weather_enabled(true);
    game.cycle_weather();
    for step in 0..1500 {
        game.update(step as f32 * 0.02);
    }
    game
}

#[test]
fn snow_falls_in_taiga_instead_of_rain() {
    let game = game_at(Biome::Taiga);
    assert_eq!(game.weather(), Weather::Rain);
    assert!(game.rain_drops() > 0.5);
    assert!(game.snowing());
}

#[test]
fn rain_falls_elsewhere_and_the_desert_stays_dry() {
    for biome in [Biome::Plains, Biome::Forest] {
        let game = game_at(biome);
        assert!(game.rain_drops() > 0.5, "{biome:?}");
        assert!(!game.snowing(), "{biome:?}");
    }
    let desert = game_at(Biome::Desert);
    assert_eq!(desert.rain_drops(), 0.0);
    assert!(!desert.snowing());
}

#[test]
fn no_snow_in_clear_weather() {
    let (x, z) = find(Biome::Taiga);
    let game = Game::new(SEED, Some(&[x as f32, 150.0, z as f32, 0.0, 0.0, 1.0])).unwrap();
    assert!(!game.snowing());
    assert_eq!(game.rain_drops(), 0.0);
}

#[test]
fn thunderstorm_has_no_flashes_in_taiga_but_does_elsewhere() {
    let flashes = |biome| {
        let mut game = game_at(biome);
        game.cycle_weather();
        assert_eq!(game.weather(), Weather::Thunder);
        let mut peak: f32 = 0.0;
        for step in 0..30_000 {
            game.update(30.0 + step as f32 * 0.02);
            peak = peak.max(game.flash());
        }
        peak
    };
    assert_eq!(flashes(Biome::Taiga), 0.0);
    assert!(flashes(Biome::Plains) > 0.5);
}
