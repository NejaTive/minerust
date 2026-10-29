use crate::physics::TICK;

pub const DAY_SECONDS: f32 = 1200.0;

pub const TICKS_PER_DAY: u32 = (DAY_SECONDS / TICK) as u32;

pub const DAY_END: f32 = 0.5;
pub const DUSK_END: f32 = 0.575;
pub const NIGHT_END: f32 = 0.925;

pub const NOON: f32 = 0.25;

pub const MORNING: f32 = 0.02;

pub const NIGHT_DARKNESS: f32 = 11.0;

const NIGHT_UNDERWATER: f32 = 0.12;

const DAY_SKY: [f32; 3] = [0.15, 0.52, 0.97];
pub const DUSK_SKY: [f32; 3] = [0.78, 0.32, 0.14];
const NIGHT_SKY: [f32; 3] = [0.012, 0.02, 0.07];

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

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SkyState {
    pub darkness: f32,
    pub color: [f32; 3],
    pub underwater_scale: f32,
    pub zenith: [f32; 3],
    pub night: f32,
    pub warm: f32,
}

const SUNSET: f32 = (DAY_END + DUSK_END) / 2.0;
const SUNRISE: f32 = (NIGHT_END + 1.0) / 2.0;

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

pub fn sun_direction(time: f32) -> [f32; 3] {
    let angle = sun_angle(time);
    [angle.cos(), angle.sin(), 0.0]
}

pub fn star_rotation(time: f32) -> [f32; 2] {
    let angle = std::f32::consts::TAU * time.rem_euclid(1.0);
    [angle.cos(), angle.sin()]
}

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

pub fn sky_state(time: f32) -> SkyState {
    let (night, warm) = night_and_dusk(time);
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
