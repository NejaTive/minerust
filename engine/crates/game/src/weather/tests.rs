use super::*;
use crate::daytime::sky_state;

const MINUTE: u32 = MINUTE_TICKS;

fn history(seed: i32, changes: usize) -> Vec<(Weather, u32)> {
    let mut state = WeatherState::new(seed);
    let mut runs = Vec::new();
    let mut current = (state.kind(), 0);
    while runs.len() < changes {
        state.advance(1, 60, true);
        if state.kind() == current.0 {
            current.1 += 60;
        } else {
            runs.push(current);
            current = (state.kind(), 0);
        }
    }
    runs
}

#[test]
fn starts_clear_and_dry() {
    let state = WeatherState::new(5);
    assert_eq!(state.kind(), Weather::Clear);
    assert_eq!(
        (state.rain(), state.thunder(), state.flash()),
        (0.0, 0.0, 0.0)
    );
}

#[test]
fn same_seed_same_sequence_other_seed_differs() {
    assert_eq!(history(7, 20), history(7, 20));
    assert_ne!(history(7, 20), history(8, 20));
}

#[test]
fn clear_and_rain_last_within_ranges() {
    for (kind, ticks) in history(3, 60).into_iter().skip(1) {
        let minutes = ticks / MINUTE;
        match kind {
            Weather::Clear => assert!((9..=21).contains(&minutes), "clear {minutes}"),
            _ => assert!((2..=9).contains(&minutes), "{kind:?} {minutes}"),
        }
    }
}

#[test]
fn clear_alternates_with_precipitation_and_a_third_is_thunder() {
    let runs = history(11, 400);
    for pair in runs.windows(2) {
        assert_eq!(pair[0].0 == Weather::Clear, pair[1].0 != Weather::Clear);
    }
    let rainy: Vec<_> = runs.iter().filter(|r| r.0 != Weather::Clear).collect();
    let thunder = rainy.iter().filter(|r| r.0 == Weather::Thunder).count();
    let share = thunder as f32 / rainy.len() as f32;
    assert!((0.2..=0.47).contains(&share), "{share}");
}

#[test]
fn rain_ramps_smoothly_over_transition() {
    let mut state = WeatherState::new(1);
    state.set_kind(Weather::Rain);
    let ticks = (TRANSITION_SECONDS / TICK) as u32;
    let mut previous = 0.0;
    for tick in 1..=ticks {
        state.advance(1, 1, true);
        assert!(state.rain() > previous && state.rain() - previous < 0.01);
        previous = state.rain();
        if tick == ticks / 2 {
            assert!((state.rain() - 0.5).abs() < 0.01);
        }
    }
    state.advance(2, 1, true);
    assert_eq!(state.rain(), 1.0);
    assert_eq!(state.thunder(), 0.0);
    state.set_kind(Weather::Clear);
    state.advance(ticks / 2, 1, true);
    assert!((state.rain() - 0.5).abs() < 0.01);
}

#[test]
fn thunder_ramps_and_stays_with_rain() {
    let mut state = WeatherState::new(1);
    state.set_kind(Weather::Thunder);
    state.advance(2000, 1, true);
    assert_eq!((state.rain(), state.thunder()), (1.0, 1.0));
    state.set_kind(Weather::Rain);
    state.advance(2000, 1, true);
    assert_eq!((state.rain(), state.thunder()), (1.0, 0.0));
}

#[test]
fn pace_speeds_up_the_timer_and_zero_stops_it() {
    let mut slow = WeatherState::new(2);
    let mut fast = slow.clone();
    let mut frozen = slow.clone();
    for _ in 0..100 {
        slow.advance(1, 1, true);
        fast.advance(1, 60, true);
        frozen.advance(1, 0, true);
    }
    let remaining = |s: &WeatherState| s.to_state()[1] as u32;
    let start = remaining(&WeatherState::new(2));
    assert_eq!(start - remaining(&slow), 100);
    assert_eq!(start - remaining(&fast), 6000);
    assert_eq!(remaining(&frozen), start);
}

#[test]
fn no_ticks_change_nothing() {
    let mut state = WeatherState::new(2);
    state.set_kind(Weather::Thunder);
    state.advance(300, 1, true);
    let before = state.clone();
    state.advance(0, 60, true);
    assert_eq!(state, before);
}

#[test]
fn disabled_weather_clears_up_and_stays_clear() {
    let mut state = WeatherState::new(2);
    state.set_kind(Weather::Thunder);
    state.advance(2000, 1, true);
    state.advance(1000, 60, false);
    assert_eq!((state.rain(), state.thunder()), (0.0, 0.0));
    assert_eq!(state.kind(), Weather::Thunder);
    assert_eq!(state.to_state()[1] as u32, state.remaining);
    let remaining = state.remaining;
    state.advance(1000, 60, false);
    assert_eq!(state.remaining, remaining);
}

#[test]
fn lightning_only_in_thunder_with_bounded_gaps() {
    let mut rain = WeatherState::new(4);
    rain.set_kind(Weather::Rain);
    let mut storm = WeatherState::new(4);
    storm.set_kind(Weather::Thunder);
    let mut flash_starts = Vec::new();
    let mut lit_before = false;
    for tick in 0..(120.0 / TICK) as u32 {
        rain.advance(1, 1, true);
        storm.advance(1, 1, true);
        assert_eq!(rain.flash(), 0.0);
        let lit = storm.flash() > 0.0;
        if lit && !lit_before {
            flash_starts.push(tick);
        }
        lit_before = lit;
        assert!(storm.flash() <= 1.0);
    }
    assert!(flash_starts.len() >= 5, "{flash_starts:?}");
    let gaps: Vec<f32> = flash_starts
        .windows(2)
        .map(|w| (w[1] - w[0]) as f32 * TICK)
        .collect();
    assert!(gaps.iter().all(|g| (5.0..=20.5).contains(g)), "{gaps:?}");
    assert!(flash_starts[0] as f32 * TICK >= TRANSITION_SECONDS * 0.5);
}

#[test]
fn flash_lasts_a_fraction_of_a_second() {
    let mut storm = WeatherState::new(9);
    storm.set_kind(Weather::Thunder);
    let mut lit_ticks = 0;
    let mut seen = false;
    for _ in 0..(40.0 / TICK) as u32 {
        storm.advance(1, 1, true);
        if storm.flash() > 0.0 {
            seen = true;
            lit_ticks += 1;
        } else if seen {
            break;
        }
    }
    assert_eq!(lit_ticks, FLASH_TICKS);
}

#[test]
fn state_roundtrips() {
    let mut state = WeatherState::new(6);
    state.set_kind(Weather::Rain);
    state.advance(300, 5, true);
    let restored = WeatherState::from_state(6, &state.to_state()).unwrap();
    assert_eq!(restored.kind(), state.kind());
    assert_eq!(restored.rain(), state.rain());
    assert_eq!(restored.thunder(), state.thunder());
    assert_eq!(restored.to_state(), state.to_state());
    let mut a = state.clone();
    let mut b = restored;
    a.advance(100_000, 60, true);
    b.advance(100_000, 60, true);
    assert_eq!(a.kind(), b.kind());
}

#[test]
fn bad_state_is_rejected() {
    let good = WeatherState::new(1).to_state();
    assert!(WeatherState::from_state(1, &good[..4]).is_none());
    for index in 0..good.len() {
        for bad in [f32::NAN, f32::INFINITY, -1.0, 0.5] {
            let mut state = good.clone();
            state[index] = bad;
            let accepted = WeatherState::from_state(1, &state).is_some();
            let expected = (index == 2 || index == 3) && bad.is_finite();
            assert_eq!(accepted, expected, "index {index} value {bad}");
        }
    }
    let mut unknown_kind = good;
    unknown_kind[0] = 3.0;
    assert!(WeatherState::from_state(1, &unknown_kind).is_none());
}

#[test]
fn restored_levels_are_clamped() {
    let mut state = WeatherState::new(1).to_state();
    state[2] = 4.0;
    state[3] = 0.0;
    assert_eq!(WeatherState::from_state(1, &state).unwrap().rain(), 1.0);
}

#[test]
fn rain_greys_and_darkens_the_sky() {
    let noon = sky_state(0.25);
    let rainy = tint_sky(noon, 1.0, 0.0);
    let spread =
        |c: [f32; 3]| c.iter().cloned().fold(0.0, f32::max) - c.iter().cloned().fold(1.0, f32::min);
    assert!(spread(rainy.color) < spread(noon.color) * 0.3);
    assert!(rainy.color.iter().sum::<f32>() < noon.color.iter().sum::<f32>());
    assert!(tint_sky(noon, 0.0, 0.0) == noon);
}

#[test]
fn flash_brightens_the_sky() {
    let night = sky_state(0.75);
    let lit = tint_sky(night, 1.0, 1.0);
    assert!(lit.color.iter().sum::<f32>() > 5.0 * night.color.iter().sum::<f32>());
    assert!(lit.zenith.iter().sum::<f32>() > night.zenith.iter().sum::<f32>());
}
