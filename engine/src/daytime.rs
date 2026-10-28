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

/// Линейные цвета неба и тумана. День — прежний `SKY` движка (sRGB 0.53, 0.81, 0.92).
const DAY_SKY: [f32; 3] = [0.24, 0.62, 0.83];
const DUSK_SKY: [f32; 3] = [0.78, 0.32, 0.14];
const NIGHT_SKY: [f32; 3] = [0.012, 0.02, 0.07];

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
