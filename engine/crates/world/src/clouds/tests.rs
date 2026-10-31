use super::*;

fn fraction_at(mask: &CloudMask, coverage: f32) -> f32 {
    let threshold = cloud_threshold(coverage);
    mask.cells().iter().filter(|&&c| c >= threshold).count() as f32 / mask.cells().len() as f32
}

fn fraction(mask: &CloudMask) -> f32 {
    fraction_at(mask, COVERAGE)
}

#[test]
fn same_seed_same_mask() {
    assert_eq!(CloudMask::new(7).cells(), CloudMask::new(7).cells());
}

#[test]
fn different_seeds_differ() {
    assert_ne!(CloudMask::new(1).cells(), CloudMask::new(2).cells());
}

#[test]
fn coverage_is_within_bounds() {
    for seed in [0, 1, -5, 123_456] {
        let f = fraction(&CloudMask::new(seed));
        assert!((0.3..=0.4).contains(&f), "seed {seed}: {f}");
    }
}

#[test]
fn cells_hold_uniform_ranks() {
    let mut counts = [0; 256];
    for &c in CloudMask::new(3).cells() {
        counts[usize::from(c)] += 1;
    }
    assert!(counts.iter().all(|&n| n == CLOUD_MAP * CLOUD_MAP / 256));
}

#[test]
fn rain_raises_coverage_through_threshold() {
    let mask = CloudMask::new(5);
    assert!(cloud_threshold(coverage_for_rain(1.0)) < cloud_threshold(coverage_for_rain(0.0)));
    assert_eq!(coverage_for_rain(0.0), COVERAGE);
    assert_eq!(coverage_for_rain(1.0), RAIN_COVERAGE);
    let rainy = fraction_at(&mask, coverage_for_rain(1.0));
    assert!((0.65..=0.75).contains(&rainy), "{rainy}");
}

#[test]
fn indexing_wraps_with_negative_coordinates() {
    let mask = CloudMask::new(11);
    let n = CLOUD_MAP as i32;
    for (x, z) in [(0, 0), (5, 17), (255, 255), (100, 3)] {
        assert_eq!(mask.get(x, z), mask.get(x - n, z));
        assert_eq!(mask.get(x, z), mask.get(x, z + 3 * n));
        assert_eq!(mask.get(x, z), mask.get(x - 7 * n, z - n));
    }
    assert_eq!(mask.get(-1, -1), mask.get(n - 1, n - 1));
}

#[test]
fn mask_has_patches_of_varied_size() {
    let mask = CloudMask::new(5);
    let transitions = (0..CLOUD_MAP as i32)
        .filter(|&x| mask.get(x, 40) != mask.get(x + 1, 40))
        .count();
    assert!((4..=100).contains(&transitions), "{transitions}");
}

#[test]
fn cloud_drift_moves_at_speed_and_wraps_at_map_size() {
    let period = (CLOUD_MAP as i32 * CLOUD_CELL) as f64;
    assert_eq!(cloud_drift(10.0), 10.0);
    assert_eq!(cloud_drift(period + 5.0), 5.0);
}
