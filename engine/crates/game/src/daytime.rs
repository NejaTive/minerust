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
mod tests;
