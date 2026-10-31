use std::ops::RangeInclusive;

use crate::daytime::SkyState;
use crate::physics::TICK;

pub const TRANSITION_SECONDS: f32 = 12.0;
pub const FLASH_SECONDS: f32 = 0.15;

const MINUTE_TICKS: u32 = (60.0 / TICK) as u32;
const CLEAR_MINUTES: RangeInclusive<u32> = 10..=20;
const PRECIPITATION_MINUTES: RangeInclusive<u32> = 3..=8;
const THUNDER_ONE_IN: u64 = 3;
const FLASH_GAP_SECONDS: RangeInclusive<u32> = 5..=20;
const FLASH_TICKS: u32 = (FLASH_SECONDS / TICK) as u32;

const RAIN_GREY: f32 = 0.85;
const RAIN_DARKEN: f32 = 0.6;
const RAIN_WARM_LOSS: f32 = 0.8;
const FLASH_COLOR: [f32; 3] = [0.8, 0.85, 1.0];
const FLASH_SKY_MIX: f32 = 0.75;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Weather {
    Clear,
    Rain,
    Thunder,
}

impl Weather {
    pub fn name(self) -> &'static str {
        match self {
            Self::Clear => "clear",
            Self::Rain => "rain",
            Self::Thunder => "thunder",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Clear => Self::Rain,
            Self::Rain => Self::Thunder,
            Self::Thunder => Self::Clear,
        }
    }

    fn index(self) -> f32 {
        match self {
            Self::Clear => 0.0,
            Self::Rain => 1.0,
            Self::Thunder => 2.0,
        }
    }

    fn from_index(index: f32) -> Option<Self> {
        match index {
            0.0 => Some(Self::Clear),
            1.0 => Some(Self::Rain),
            2.0 => Some(Self::Thunder),
            _ => None,
        }
    }
}

fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

fn draw(seed: i32, counter: u32, salt: u64) -> u64 {
    mix64(mix64(u64::from(seed as u32) ^ (salt << 40)) ^ u64::from(counter))
}

fn in_range(roll: u64, range: &RangeInclusive<u32>) -> u32 {
    let (lo, hi) = (*range.start(), *range.end());
    lo + (roll % u64::from(hi - lo + 1)) as u32
}

fn move_towards(value: f32, target: f32, step: f32) -> f32 {
    if value < target {
        (value + step).min(target)
    } else {
        (value - step).max(target)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WeatherState {
    seed: i32,
    kind: Weather,
    remaining: u32,
    changes: u32,
    rain: f32,
    thunder: f32,
    flash_gap: u32,
    flash_left: u32,
    flashes: u32,
}

impl WeatherState {
    pub fn new(seed: i32) -> Self {
        let mut state = Self {
            seed,
            kind: Weather::Clear,
            remaining: 0,
            changes: 0,
            rain: 0.0,
            thunder: 0.0,
            flash_gap: 0,
            flash_left: 0,
            flashes: 0,
        };
        state.begin(Weather::Clear);
        state
    }

    pub fn kind(&self) -> Weather {
        self.kind
    }

    pub fn rain(&self) -> f32 {
        self.rain
    }

    pub fn thunder(&self) -> f32 {
        self.thunder
    }

    pub fn flash(&self) -> f32 {
        self.flash_left as f32 / FLASH_TICKS as f32
    }

    pub fn set_kind(&mut self, kind: Weather) {
        self.changes += 1;
        self.begin(kind);
    }

    fn begin(&mut self, kind: Weather) {
        self.kind = kind;
        let range = match kind {
            Weather::Clear => &CLEAR_MINUTES,
            _ => &PRECIPITATION_MINUTES,
        };
        self.remaining = in_range(draw(self.seed, self.changes, 1), range) * MINUTE_TICKS;
        self.schedule_flash();
    }

    fn schedule_flash(&mut self) {
        let seconds = in_range(draw(self.seed, self.flashes, 2), &FLASH_GAP_SECONDS);
        self.flashes += 1;
        self.flash_gap = (seconds as f32 / TICK) as u32;
    }

    fn roll_next(&mut self) {
        self.changes += 1;
        let next = match self.kind {
            Weather::Clear if draw(self.seed, self.changes, 3).is_multiple_of(THUNDER_ONE_IN) => {
                Weather::Thunder
            }
            Weather::Clear => Weather::Rain,
            _ => Weather::Clear,
        };
        self.begin(next);
    }

    pub fn advance(&mut self, ticks: u32, pace: u32, enabled: bool) {
        if enabled {
            let mut elapsed = u64::from(ticks) * u64::from(pace);
            while elapsed >= u64::from(self.remaining) {
                elapsed -= u64::from(self.remaining);
                self.roll_next();
            }
            self.remaining -= elapsed as u32;
        }
        let step = ticks as f32 * TICK / TRANSITION_SECONDS;
        let active = enabled && self.kind != Weather::Clear;
        self.rain = move_towards(self.rain, f32::from(u8::from(active)), step);
        let stormy = enabled && self.kind == Weather::Thunder;
        self.thunder = move_towards(self.thunder, f32::from(u8::from(stormy)), step);
        for _ in 0..ticks {
            self.flash_left = self.flash_left.saturating_sub(1);
            if stormy && self.thunder > 0.5 {
                self.flash_gap = self.flash_gap.saturating_sub(1);
                if self.flash_gap == 0 {
                    self.flash_left = FLASH_TICKS;
                    self.schedule_flash();
                }
            }
        }
    }

    pub fn to_state(&self) -> Vec<f32> {
        vec![
            self.kind.index(),
            self.remaining as f32,
            self.rain,
            self.thunder,
            self.changes as f32,
        ]
    }

    pub fn from_state(seed: i32, state: &[f32]) -> Option<Self> {
        let &[kind, remaining, rain, thunder, changes] = state else {
            return None;
        };
        let valid = |v: f32| v.is_finite() && v >= 0.0 && v.fract() == 0.0;
        if !valid(remaining) || !valid(changes) || !rain.is_finite() || !thunder.is_finite() {
            return None;
        }
        let mut restored = Self::new(seed);
        restored.kind = Weather::from_index(kind)?;
        restored.remaining = (remaining as u32).max(1);
        restored.changes = changes as u32;
        restored.rain = rain.clamp(0.0, 1.0);
        restored.thunder = thunder.clamp(0.0, 1.0);
        Some(restored)
    }
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

fn stormy(color: [f32; 3], rain: f32, flash: f32) -> [f32; 3] {
    let luma = 0.299 * color[0] + 0.587 * color[1] + 0.114 * color[2];
    let grey = [luma * RAIN_DARKEN; 3];
    let dull = mix(color, grey, rain * RAIN_GREY);
    mix(dull, FLASH_COLOR, flash * FLASH_SKY_MIX)
}

pub fn tint_sky(sky: SkyState, rain: f32, flash: f32) -> SkyState {
    SkyState {
        color: stormy(sky.color, rain, flash),
        zenith: stormy(sky.zenith, rain, flash),
        warm: sky.warm * (1.0 - RAIN_WARM_LOSS * rain),
        ..sky
    }
}

#[cfg(test)]
mod tests;
