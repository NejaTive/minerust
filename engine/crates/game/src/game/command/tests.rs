use world::terrain::{Biome, WorldType};

use super::super::Game;

const SEED: i32 = 21;

fn game() -> Game {
    Game::new(SEED, None).unwrap()
}

fn ok(game: &mut Game, line: &str) -> String {
    let out = game.run_command(line);
    assert!(out.ok, "{line}: {}", out.text);
    out.text
}

fn err(game: &mut Game, line: &str) -> String {
    let out = game.run_command(line);
    assert!(!out.ok, "{line}: {}", out.text);
    out.text
}

fn ticks(game: &Game) -> u32 {
    game.day.mc_ticks()
}

#[test]
fn time_set_names_and_numbers() {
    let mut g = game();
    for (line, expected) in [
        ("/time set day", 1000),
        ("/time set noon", 6000),
        ("/time set night", 13000),
        ("/time set midnight", 18000),
        ("time set 123", 123),
        ("/time set 24500", 500),
    ] {
        ok(&mut g, line);
        assert_eq!(ticks(&g), expected, "{line}");
    }
}

#[test]
fn time_noon_matches_engine_noon() {
    let mut g = game();
    ok(&mut g, "/time set noon");
    assert!((g.time_of_day() - crate::daytime::NOON).abs() < 1e-6);
}

#[test]
fn time_add_wraps() {
    let mut g = game();
    ok(&mut g, "/time set 23000");
    ok(&mut g, "/time add 2000");
    assert_eq!(ticks(&g), 1000);
    ok(&mut g, "/time add -1500");
    assert_eq!(ticks(&g), 23500);
}

#[test]
fn time_query_daytime() {
    let mut g = game();
    ok(&mut g, "/time set 6000");
    assert!(ok(&mut g, "/time query daytime").contains("6000"));
}

#[test]
fn time_errors_show_usage() {
    let mut g = game();
    for line in [
        "/time",
        "/time set",
        "/time set abc",
        "/time set -5",
        "/time add x",
        "/time query gametime",
        "/time query day",
        "/time bogus",
    ] {
        assert!(err(&mut g, line).contains("/time"), "{line}");
    }
}

#[test]
fn weather_sets_kind_and_duration() {
    let mut g = game();
    ok(&mut g, "/weather rain");
    assert_eq!(g.weather().name(), "rain");
    ok(&mut g, "/weather thunder 30");
    assert_eq!(g.weather().name(), "thunder");
    ok(&mut g, "/weather clear");
    assert_eq!(g.weather().name(), "clear");
}

#[test]
fn weather_duration_is_applied() {
    let mut g = game();
    ok(&mut g, "/weather rain 10");
    assert_eq!(g.weather_state()[1], 600.0);
}

#[test]
fn weather_errors() {
    let mut g = game();
    for line in [
        "/weather",
        "/weather snow",
        "/weather rain 0",
        "/weather rain x",
        "/weather rain 1 2",
    ] {
        assert!(err(&mut g, line).contains("/weather"), "{line}");
    }
    g.set_weather_enabled(false);
    assert!(err(&mut g, "/weather rain").contains("выключена"));
    assert_eq!(g.weather().name(), "clear");
}

#[test]
fn locate_finds_each_biome_at_reported_spot() {
    let mut g = game();
    for (name, biome) in [
        ("plains", Biome::Plains),
        ("desert", Biome::Desert),
        ("forest", Biome::Forest),
        ("taiga", Biome::Taiga),
        ("ice_plains", Biome::IcePlains),
        ("ocean", Biome::Ocean),
        ("river", Biome::River),
        ("beach", Biome::Beach),
    ] {
        let out = g.run_command(&format!("/locate biome {name}"));
        if !out.ok {
            assert!(out.text.contains("не найден"), "{name}: {}", out.text);
            continue;
        }
        let inner = out
            .text
            .split('[')
            .nth(1)
            .unwrap()
            .split(']')
            .next()
            .unwrap();
        let mut it = inner.split(", ");
        let x: i32 = it.next().unwrap().parse().unwrap();
        let z: i32 = it.nth(1).unwrap().parse().unwrap();
        assert_eq!(g.terrain.biome_at(x, z), biome, "{name}");
    }
}

#[test]
fn locate_current_biome_is_distance_zero() {
    let mut g = game();
    let eye = g.player().eye().floor().as_ivec3();
    let here = g.terrain.biome_at(eye.x, eye.z);
    assert!(ok(&mut g, &format!("/locate biome {}", here.name())).contains("(0 блоков)"));
}

#[test]
fn locate_in_debug_world_reports_not_found() {
    let mut g = Game::of(SEED, WorldType::Debug, None).unwrap();
    let here = {
        let eye = g.player().eye().floor().as_ivec3();
        g.terrain.biome_at(eye.x, eye.z)
    };
    let other = if here == Biome::Forest {
        "plains"
    } else {
        "forest"
    };
    assert!(err(&mut g, &format!("/locate biome {other}")).contains("не найден"));
}

#[test]
fn locate_errors() {
    let mut g = game();
    for line in [
        "/locate",
        "/locate biome",
        "/locate biome atlantis",
        "/locate structure x",
    ] {
        assert!(err(&mut g, line).contains("/locate"), "{line}");
    }
}

#[test]
fn help_lists_and_describes() {
    let mut g = game();
    let list = ok(&mut g, "/help");
    for name in ["/time", "/weather", "/locate", "/help"] {
        assert!(list.contains(name));
    }
    assert!(ok(&mut g, "/help weather").contains("/weather <clear|rain|thunder>"));
    assert!(err(&mut g, "/help nope").contains("Неизвестная команда"));
}

#[test]
fn unknown_and_empty_commands() {
    let mut g = game();
    assert!(err(&mut g, "/fly").contains("Неизвестная команда"));
    assert!(err(&mut g, "/").contains("Введите команду"));
}

#[test]
fn complete_commands_and_arguments() {
    let g = game();
    let c = g.complete_command("/");
    assert_eq!((c.start, c.options.len()), (1, 6));
    let c = g.complete_command("/ti");
    assert_eq!((c.start, c.options), (1, vec!["time".to_owned()]));
    let c = g.complete_command("/time ");
    assert_eq!(
        (c.start, c.options),
        (6, vec!["set".into(), "add".into(), "query".into()])
    );
    let c = g.complete_command("/time set n");
    assert_eq!(
        (c.start, c.options),
        (10, vec!["noon".to_owned(), "night".to_owned()])
    );
    let c = g.complete_command("/weather ra");
    assert_eq!(c.options, vec!["rain".to_owned()]);
    let c = g.complete_command("/locate biome ta");
    assert_eq!(
        (c.start, c.options),
        (
            14,
            ["taiga", "taiga_hills", "taiga_m"]
                .map(str::to_owned)
                .to_vec()
        )
    );
    assert!(g.complete_command("/time add ").options.is_empty());
    assert!(g.complete_command("/time set zzz").options.is_empty());
}

#[test]
fn usage_hint_follows_command() {
    let g = game();
    assert!(g.command_usage("/weather rain").starts_with("/weather"));
    assert!(g.command_usage("/t").contains("/time"));
    assert!(g.command_usage("/").contains("/help"));
}

fn feet(game: &Game) -> (f32, f32, f32) {
    let f = game.player().feet;
    (f.x, f.y, f.z)
}

#[test]
fn tp_absolute_centers_integers() {
    let mut g = game();
    assert_eq!(
        ok(&mut g, "/tp 10 70 -4"),
        "Телепортация на [10.5, 70, -3.5]"
    );
    assert_eq!(feet(&g), (10.5, 70.0, -3.5));
    ok(&mut g, "/tp 1.25 80 2.75");
    assert_eq!(feet(&g), (1.25, 80.0, 2.75));
}

#[test]
fn tp_relative_offsets() {
    let mut g = game();
    ok(&mut g, "/tp 0 100 0");
    ok(&mut g, "/tp ~3 ~-10 ~");
    assert_eq!(feet(&g), (3.5, 90.0, 0.5));
    ok(&mut g, "/tp ~ ~ ~5");
    assert_eq!(feet(&g), (3.5, 90.0, 5.5));
}

#[test]
fn tp_tilde_height_with_absolute_xz_uses_surface() {
    let mut g = game();
    ok(&mut g, "/tp 300 ~ -200");
    let surface = g.terrain.surface_height(300, -200);
    assert_eq!(feet(&g), (300.5, (surface + 1) as f32, -199.5));
}

#[test]
fn tp_resets_velocity() {
    let mut g = game();
    g.player.velocity = glam::Vec3::new(1.0, -30.0, 2.0);
    ok(&mut g, "/tp 0 80 0");
    assert_eq!(g.player().velocity, glam::Vec3::ZERO);
}

#[test]
fn tp_errors() {
    let mut g = game();
    for line in [
        "/tp",
        "/tp 1 2",
        "/tp a b c",
        "/tp 1 2 3 4",
        "/tp 0 -5 0",
        "/tp 0 9999 0",
        "/tp 1e9 70 0",
    ] {
        assert!(err(&mut g, line).contains("/tp"), "{line}");
    }
}

#[test]
fn tp_completes_tilde() {
    let g = game();
    assert_eq!(g.complete_command("/tp ").options, vec!["~".to_owned()]);
    assert_eq!(g.complete_command("/tp 1 ~").start, 6);
    assert!(g.complete_command("/tp 1 2 3 ").options.is_empty());
    assert!(g.complete_command("/").options.contains(&"tp".to_owned()));
}

#[test]
fn locate_suggests_tp() {
    let mut g = game();
    let out = g.run_command("/locate biome desert");
    if out.ok {
        let suggest = out.suggest.unwrap();
        let coords = out
            .text
            .split('[')
            .nth(1)
            .unwrap()
            .split(']')
            .next()
            .unwrap();
        let mut it = coords.split(", ");
        let (x, z) = (it.next().unwrap(), it.nth(1).unwrap());
        assert_eq!(suggest, format!("/tp {x} ~ {z}"));
    }
    assert!(g.run_command("/time set day").suggest.is_none());
}

#[test]
fn teleported_player_waits_for_unloaded_columns() {
    let mut g = game();
    ok(&mut g, "/tp 5000 ~ 5000");
    let before = feet(&g);
    for frame in 1..=120 {
        g.update(frame as f32 * 0.016);
    }
    assert_eq!(feet(&g), before);
}

#[test]
fn speed_sets_and_reports() {
    let mut g = game();
    assert_eq!(ok(&mut g, "/speed walk 5"), "Скорость ходьбы: ×5");
    assert_eq!(g.player().walk_multiplier, 5.0);
    assert_eq!(ok(&mut g, "/speed fly 0.5"), "Скорость полёта: ×0.5");
    assert_eq!(g.player().fly_multiplier, 0.5);
    assert_eq!(ok(&mut g, "/speed walk"), "Скорость ходьбы: ×5");
    assert_eq!(ok(&mut g, "/speed fly 10"), "Скорость полёта: ×10");
    assert_eq!(ok(&mut g, "/speed fly 0.1"), "Скорость полёта: ×0.1");
}

#[test]
fn speed_errors_keep_value() {
    let mut g = game();
    for line in [
        "/speed",
        "/speed run 2",
        "/speed walk 0",
        "/speed walk 10.1",
        "/speed walk -1",
        "/speed walk x",
        "/speed walk nan",
        "/speed walk 1 2",
    ] {
        assert!(err(&mut g, line).contains("/speed"), "{line}");
    }
    assert_eq!(g.player().walk_multiplier, 1.0);
}

#[test]
fn speed_completes_modes() {
    let g = game();
    assert_eq!(
        g.complete_command("/speed ").options,
        vec!["walk".to_owned(), "fly".to_owned()]
    );
    assert!(
        g.complete_command("/s")
            .options
            .contains(&"speed".to_owned())
    );
}

#[test]
fn every_biome_has_a_locate_name_and_a_label() {
    for &biome in Biome::ALL {
        assert_eq!(super::biome_entry(biome.name()), Some(biome));
        assert!(!super::biome_label(biome).is_empty());
    }
}
