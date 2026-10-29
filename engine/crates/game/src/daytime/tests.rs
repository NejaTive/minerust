use super::*;

#[test]
fn day_is_unchanged() {
    for t in [0.0, MORNING, NOON, DAY_END - 0.001] {
        let s = sky_state(t);
        assert_eq!(s.darkness, 0.0, "{t}");
        assert_eq!(s.color, DAY_SKY);
        assert_eq!(s.underwater_scale, 1.0);
    }
}

#[test]
fn night_is_dark_blue() {
    for t in [DUSK_END, 0.75, NIGHT_END - 0.001] {
        let s = sky_state(t);
        assert_eq!(s.darkness, NIGHT_DARKNESS);
        for (got, want) in s.color.iter().zip(NIGHT_SKY) {
            assert!((got - want).abs() < 1e-6);
        }
        assert!((s.underwater_scale - NIGHT_UNDERWATER).abs() < 1e-6);
    }
}

#[test]
fn phase_boundaries() {
    assert_eq!(phase(0.0), Phase::Day);
    assert_eq!(phase(DAY_END - 1e-4), Phase::Day);
    assert_eq!(phase(DAY_END), Phase::Dusk);
    assert_eq!(phase(DUSK_END), Phase::Night);
    assert_eq!(phase(NIGHT_END), Phase::Dawn);
    assert_eq!(phase(1.0 - 1e-4), Phase::Dawn);
    assert_eq!(phase(1.0), Phase::Day);
}

#[test]
fn dusk_is_warm_in_the_middle() {
    let mid = sky_state((DAY_END + DUSK_END) / 2.0).color;
    assert!(mid[0] > mid[2], "красный сильнее синего: {mid:?}");
    assert!(mid[0] > DAY_SKY[0]);
    let dawn = sky_state((NIGHT_END + 1.0) / 2.0).color;
    assert!(dawn[0] > dawn[2], "{dawn:?}");
}

#[test]
fn transitions_are_continuous() {
    let n = 20_000;
    let mut prev = sky_state(0.0);
    for i in 1..=n + 1 {
        let s = sky_state(i as f32 / n as f32);
        assert!((s.darkness - prev.darkness).abs() < 0.02, "{i}");
        assert!(
            (s.underwater_scale - prev.underwater_scale).abs() < 0.001,
            "{i}"
        );
        for c in 0..3 {
            assert!((s.color[c] - prev.color[c]).abs() < 0.002, "{i} {c}");
        }
        prev = s;
    }
}

#[test]
fn sun_rises_east_peaks_at_noon_sets_west() {
    let close = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-4);
    assert!(close(sun_direction(SUNRISE), [1.0, 0.0, 0.0]));
    assert!(close(sun_direction(NOON), [0.0, 1.0, 0.0]));
    assert!(close(sun_direction(SUNSET), [-1.0, 0.0, 0.0]));
    assert!(close(sun_direction(1.0), sun_direction(0.0)));
}

#[test]
fn sun_is_up_by_day_and_down_by_night() {
    for t in [0.0, 0.02, 0.1, NOON, 0.4, 0.53] {
        assert!(sun_direction(t)[1] > 0.0, "{t}");
        assert!(moon_direction(t)[1] < 0.0, "{t}");
    }
    for t in [0.55, DUSK_END, 0.75, NIGHT_END, 0.95] {
        assert!(sun_direction(t)[1] < 0.0, "{t}");
    }
    for t in [0.55, DUSK_END, 0.7, 0.75, 0.9, 0.95] {
        assert!(moon_direction(t)[1] > 0.0, "{t}");
    }
}

#[test]
fn sun_is_continuous() {
    let n = 20_000;
    let mut prev = sun_direction(0.0);
    for i in 1..=n + 1 {
        let s = sun_direction(i as f32 / n as f32);
        assert!((0..3).all(|c| (s[c] - prev[c]).abs() < 0.005), "{i}");
        prev = s;
    }
}

#[test]
fn twilight_is_centered_on_the_horizon() {
    let sunset = sun_direction((DAY_END + DUSK_END) / 2.0)[1];
    let sunrise = sun_direction((NIGHT_END + 1.0) / 2.0)[1];
    assert!(sunset.abs() < 1e-4 && sunrise.abs() < 1e-4);
}

#[test]
fn stars_turn_uniformly_and_wrap() {
    let angle = |t: f32| {
        let [c, s] = star_rotation(t);
        s.atan2(c).rem_euclid(std::f32::consts::TAU)
    };
    let step = angle(0.1) - angle(0.0);
    for t in [0.0, 0.3, 0.45, 0.6, 0.8] {
        let d = (angle(t + 0.1) - angle(t)).rem_euclid(std::f32::consts::TAU);
        assert!((d - step).abs() < 1e-4, "{t}");
    }
    assert_eq!(star_rotation(1.0), star_rotation(0.0));
}

#[test]
fn moon_is_opposite_and_unit() {
    for t in [0.0, 0.13, 0.25, 0.6, 0.9] {
        let (s, m) = (sun_direction(t), moon_direction(t));
        assert!((s[0] * m[0] + s[1] * m[1] + s[2] * m[2] + 1.0).abs() < 1e-5);
        assert!((s[0] * s[0] + s[1] * s[1] - 1.0).abs() < 1e-5);
    }
}

#[test]
fn zenith_is_darker_than_horizon_and_warm_only_at_twilight() {
    for t in [MORNING, NOON, 0.75] {
        let s = sky_state(t);
        let lum = |c: [f32; 3]| c[0] + c[1] + c[2];
        assert!(lum(s.zenith) < lum(s.color), "{t}");
        assert_eq!(s.warm, 0.0, "{t}");
    }
    assert!(sky_state((DAY_END + DUSK_END) / 2.0).warm > 0.9);
    assert_eq!(sky_state(NOON).night, 0.0);
    assert_eq!(sky_state(0.75).night, 1.0);
}

#[test]
fn cycle_wraps() {
    assert_eq!(sky_state(1.0), sky_state(0.0));
    assert_eq!(sky_state(1.25), sky_state(0.25));
    assert_eq!(sky_state(-0.25), sky_state(0.75));
}

#[test]
fn day_time_counts_ticks_and_wraps() {
    assert_eq!(TICKS_PER_DAY, 72_000);
    let mut t = DayTime::from_fraction(0.0);
    t.advance(TICKS_PER_DAY / 2);
    assert_eq!(t.fraction(), 0.5);
    t.advance(TICKS_PER_DAY / 2);
    assert_eq!(t.fraction(), 0.0);
    t.advance(u32::MAX);
    assert!(t.fraction() < 1.0);
}

#[test]
fn day_time_accepts_any_float() {
    assert_eq!(DayTime::from_fraction(1.25), DayTime::from_fraction(0.25));
    assert_eq!(DayTime::from_fraction(f32::NAN), DayTime::default());
    assert!(DayTime::from_fraction(0.999_999_9).fraction() < 1.0);
    assert!((DayTime::default().fraction() - MORNING).abs() < 1e-4);
}
