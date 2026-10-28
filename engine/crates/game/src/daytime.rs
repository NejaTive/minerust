use crate::physics::TICK;

/// Длина суток в реальных секундах.
pub const DAY_SECONDS: f32 = 1200.0;

/// Шагов физики в сутках. Время хранится целым числом шагов, а не `f32`-долей:
/// за часы игры накопление не дрейфует и не теряет шаг.
pub const TICKS_PER_DAY: u32 = (DAY_SECONDS / TICK) as u32;

/// Время суток 0..1: 0 — начало дня, дальше день, закат, ночь, рассвет и снова 0.
/// Границы фаз — доли суток: день 10 мин, закат 1.5, ночь 7, рассвет 1.5 из 20.
pub const DAY_END: f32 = 0.5;
pub const DUSK_END: f32 = 0.575;
pub const NIGHT_END: f32 = 0.925;

/// Полдень: середина дня.
pub const NOON: f32 = 0.25;

/// С этого времени начинается новый мир и старое сохранение без поля `time`: утро,
/// день уже наступил, но сутки только начались.
pub const MORNING: f32 = 0.02;

/// Сколько уровней солнечного света отнимает ночь. Оставшиеся 4 из 15 дают тёмную, но
/// различимую поверхность под открытым небом (0.8^11 ≈ 0.086 против 1.0 днём).
pub const NIGHT_DARKNESS: f32 = 11.0;

/// Во сколько раз цвет под водой темнее ночью, чем днём.
const NIGHT_UNDERWATER: f32 = 0.12;

/// Линейные цвета неба и тумана. День — насыщенный синий (sRGB ≈ 107, 190, 252) по образцу, а не бледный прежний `SKY` (0.53, 0.81, 0.92).
const DAY_SKY: [f32; 3] = [0.15, 0.52, 0.97];
pub const DUSK_SKY: [f32; 3] = [0.78, 0.32, 0.14];
const NIGHT_SKY: [f32; 3] = [0.012, 0.02, 0.07];

/// Линейный цвет неба в зените: темнее и насыщеннее, чем у горизонта (он равен цвету
/// тумана, чтобы дальний рельеф сливался с небом без границы).
const DAY_ZENITH: [f32; 3] = [0.04, 0.24, 0.78];
const NIGHT_ZENITH: [f32; 3] = [0.002, 0.004, 0.02];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Day,
    Dusk,
    Night,
    Dawn,
}

impl Phase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Dusk => "dusk",
            Self::Night => "night",
            Self::Dawn => "dawn",
        }
    }
}

/// Фаза по времени суток; границы принадлежат следующей фазе.
pub fn phase(time: f32) -> Phase {
    let t = time.rem_euclid(1.0);
    if t < DAY_END {
        Phase::Day
    } else if t < DUSK_END {
        Phase::Dusk
    } else if t < NIGHT_END {
        Phase::Night
    } else {
        Phase::Dawn
    }
}

/// Всё, что время суток меняет в кадре.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SkyState {
    /// Уровней, которые вычитаются из солнечного света в шейдере: 0 днём,
    /// [`NIGHT_DARKNESS`] ночью.
    pub darkness: f32,
    /// Линейный цвет неба, он же цвет тумана и фона кадра.
    pub color: [f32; 3],
    /// Множитель цвета тумана и фона под водой: 1 днём.
    pub underwater_scale: f32,
    /// Линейный цвет неба в зените.
    pub zenith: [f32; 3],
    /// Степень ночи 0..1: видимость звёзд.
    pub night: f32,
    /// Вес тёплого пятна у горизонта 0..1 (закат и рассвет).
    pub warm: f32,
}

/// Моменты, когда солнце на горизонте, — середины заката и рассвета: красное небо и
/// диск у горизонта совпадают. День между ними длиннее ночи (0.575 против 0.425), как и
/// фазы; [`NOON`] — середина дня.
const SUNSET: f32 = (DAY_END + DUSK_END) / 2.0;
const SUNRISE: f32 = (NIGHT_END + 1.0) / 2.0;

/// Угол солнца на дуге 0..2π: 0 — восход на +x, π — заход на −x. Дневная и ночная дуги
/// идут каждая со своей постоянной скоростью.
fn sun_angle(time: f32) -> f32 {
    use std::f32::consts::PI;
    let t = time.rem_euclid(1.0);
    let t = if t >= SUNRISE { t - 1.0 } else { t };
    if t < SUNSET {
        PI * (t - (SUNRISE - 1.0)) / (SUNSET - (SUNRISE - 1.0))
    } else {
        PI + PI * (t - SUNSET) / (SUNRISE - SUNSET)
    }
}

/// Единичное направление на солнце. Оно идёт по дуге в плоскости x–y: восход на +x при
/// [`SUNRISE`], зенит в [`NOON`], заход на −x при [`SUNSET`], дальше под горизонтом.
pub fn sun_direction(time: f32) -> [f32; 3] {
    let angle = sun_angle(time);
    [angle.cos(), angle.sin(), 0.0]
}

/// (cos, sin) поворота звёздного неба вокруг оси z: один оборот за сутки с постоянной
/// скоростью, в отличие от солнца, чья дневная и ночная дуги идут с разной.
pub fn star_rotation(time: f32) -> [f32; 2] {
    let angle = std::f32::consts::TAU * time.rem_euclid(1.0);
    [angle.cos(), angle.sin()]
}

/// Направление на луну: напротив солнца.
pub fn moon_direction(time: f32) -> [f32; 3] {
    sun_direction(time).map(|c| -c)
}

fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

/// Степень ночи 0..1 и вес тёплого оттенка на переходе: от дня к закату к ночи небо
/// идёт через тёплый цвет, дневной и ночной цвета друг с другом не смешиваются.
fn night_and_dusk(time: f32) -> (f32, f32) {
    let t = time.rem_euclid(1.0);
    let dusk_len = DUSK_END - DAY_END;
    let dawn_len = 1.0 - NIGHT_END;
    if t < DAY_END {
        (0.0, 0.0)
    } else if t < DUSK_END {
        let p = (t - DAY_END) / dusk_len;
        (smoothstep(p), 1.0 - (2.0 * p - 1.0).abs())
    } else if t < NIGHT_END {
        (1.0, 0.0)
    } else {
        let p = (t - NIGHT_END) / dawn_len;
        (1.0 - smoothstep(p), 1.0 - (2.0 * p - 1.0).abs())
    }
}

/// Параметры кадра по времени суток 0..1 (вне диапазона берётся дробная часть).
/// Все функции непрерывны, включая стык 1 → 0.
pub fn sky_state(time: f32) -> SkyState {
    let (night, warm) = night_and_dusk(time);
    // Цвет: ночь тянет к тёмно-синему, тёплый вес на переходе — к закатному.
    let base = mix(DAY_SKY, NIGHT_SKY, night);
    let color = mix(base, DUSK_SKY, smoothstep(warm) * 0.9);
    SkyState {
        darkness: NIGHT_DARKNESS * night,
        color,
        underwater_scale: 1.0 + (NIGHT_UNDERWATER - 1.0) * night,
        zenith: mix(DAY_ZENITH, NIGHT_ZENITH, night),
        night,
        warm: smoothstep(warm),
    }
}

/// Время суток в шагах физики.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DayTime(u32);

impl DayTime {
    pub fn from_fraction(time: f32) -> Self {
        let t = if time.is_finite() {
            time.rem_euclid(1.0)
        } else {
            MORNING
        };
        Self(((t * TICKS_PER_DAY as f32) as u32).min(TICKS_PER_DAY - 1))
    }

    /// Время 0..1.
    pub fn fraction(self) -> f32 {
        self.0 as f32 / TICKS_PER_DAY as f32
    }

    pub fn advance(&mut self, ticks: u32) {
        self.0 = ((u64::from(self.0) + u64::from(ticks)) % u64::from(TICKS_PER_DAY)) as u32;
    }
}

impl Default for DayTime {
    fn default() -> Self {
        Self::from_fraction(MORNING)
    }
}

#[cfg(test)]
mod tests {
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
        // Шаг в 1/20000 суток (~0.06 с): между соседними значениями разница мала,
        // в том числе на границах фаз и на стыке 1 → 0.
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
        // Тёплый пик неба — когда солнце на горизонте.
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
        // Равные шаги времени — равные углы, и днём, и ночью.
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
}
