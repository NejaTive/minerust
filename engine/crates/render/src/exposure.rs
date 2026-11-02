// Эталон расчёта для тестов: на GPU тот же расчёт в exposure.wgsl, значения держать равными.
#![cfg_attr(not(test), allow(dead_code))]

pub const FIXED_EXPOSURE: f32 = 1.2;
pub const KEY: f32 = 0.48;
pub const MIN_EXPOSURE: f32 = 0.7;
pub const MAX_EXPOSURE: f32 = 2.4;
pub const TAU_TO_DARK: f32 = 2.5;
pub const TAU_TO_LIGHT: f32 = 0.7;

pub fn target_exposure(mean_log2_luma: f32) -> f32 {
    (KEY / mean_log2_luma.exp2()).clamp(MIN_EXPOSURE, MAX_EXPOSURE)
}

fn adapt(current: f32, target: f32, dt: f32) -> f32 {
    let tau = if target > current {
        TAU_TO_DARK
    } else {
        TAU_TO_LIGHT
    };
    let blend = 1.0 - (-dt.max(0.0) / tau).exp();
    (current.log2() + (target.log2() - current.log2()) * blend).exp2()
}

pub fn next_exposure(current: f32, mean_log2_luma: f32, dt: f32, reset: bool) -> f32 {
    let target = target_exposure(mean_log2_luma);
    if reset {
        target
    } else {
        adapt(current, target, dt)
    }
}

#[cfg(test)]
mod tests;
