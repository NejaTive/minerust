use super::*;

fn fraction(mask: &CloudMask) -> f32 {
    mask.cells().iter().map(|&c| f32::from(c)).sum::<f32>() / mask.cells().len() as f32
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
fn cells_are_binary() {
    assert!(CloudMask::new(3).cells().iter().all(|&c| c <= 1));
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
