use super::*;

#[test]
fn target_is_clamped_to_range() {
    assert_eq!(target_exposure(-20.0), MAX_EXPOSURE);
    assert_eq!(target_exposure(10.0), MIN_EXPOSURE);
}

#[test]
fn target_is_key_over_luma_inside_range() {
    let luma = 0.3f32;
    let expected = KEY / luma;
    assert!((target_exposure(luma.log2()) - expected).abs() < 1e-5);
}

#[test]
fn reset_jumps_straight_to_target() {
    let luma_log2 = 0.2f32.log2();
    assert_eq!(
        next_exposure(MIN_EXPOSURE, luma_log2, 0.016, true),
        target_exposure(luma_log2)
    );
}

#[test]
fn adapting_to_dark_is_slower_than_to_light() {
    let dark = next_exposure(1.0, 0.05f32.log2(), 0.5, false);
    let light = next_exposure(2.0, 4.0f32.log2(), 0.5, false);
    assert!(dark > 1.0 && target_exposure(0.05f32.log2()) > dark);
    assert!(light < 2.0);
    let dark_progress =
        (dark.log2() - 1.0f32.log2()) / (target_exposure(0.05f32.log2()).log2() - 1.0f32.log2());
    let light_progress =
        (2.0f32.log2() - light.log2()) / (2.0f32.log2() - target_exposure(4.0f32.log2()).log2());
    assert!(light_progress > dark_progress);
}

#[test]
fn adaptation_converges_and_never_overshoots() {
    let luma_log2 = 0.1f32.log2();
    let target = target_exposure(luma_log2);
    let mut exposure = 1.0;
    for _ in 0..2000 {
        let next = next_exposure(exposure, luma_log2, 0.016, false);
        assert!(next >= exposure && next <= target);
        exposure = next;
    }
    assert!((exposure - target).abs() < 1e-3);
}

#[test]
fn zero_or_negative_dt_keeps_exposure() {
    assert_eq!(next_exposure(1.5, 0.05f32.log2(), 0.0, false), 1.5);
    assert_eq!(next_exposure(1.5, 0.05f32.log2(), -1.0, false), 1.5);
}

#[test]
fn shader_constants_match_cpu() {
    let src = crate::shaders::EXPOSURE_SHADER;
    for (name, value) in [
        ("KEY", KEY),
        ("MIN_EXPOSURE", MIN_EXPOSURE),
        ("MAX_EXPOSURE", MAX_EXPOSURE),
        ("TAU_TO_DARK", TAU_TO_DARK),
        ("TAU_TO_LIGHT", TAU_TO_LIGHT),
    ] {
        assert!(
            src.contains(&format!("const {name}: f32 = {value:?};")),
            "{name}"
        );
    }
}

#[test]
fn fixed_exposure_matches_bloom_display_threshold() {
    let threshold = crate::shaders::BLOOM_SHADER
        .lines()
        .find_map(|l| l.strip_prefix("const DISPLAY_THRESHOLD: f32 = "))
        .and_then(|l| l.strip_suffix(';'))
        .and_then(|l| l.parse::<f32>().ok())
        .expect("DISPLAY_THRESHOLD");
    assert!((threshold / FIXED_EXPOSURE - 1.05).abs() < 1e-5);
}
