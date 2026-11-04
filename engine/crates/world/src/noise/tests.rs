use super::*;

#[test]
fn octaves_are_deterministic_and_seed_dependent() {
    let a = Octaves::new(1, 8, 0);
    let b = Octaves::new(1, 8, 0);
    let c = Octaves::new(2, 8, 0);
    let point = (12.3, 4.5, -7.8);
    assert_eq!(
        a.sample(point.0, point.1, point.2),
        b.sample(point.0, point.1, point.2)
    );
    assert_ne!(
        a.sample(point.0, point.1, point.2),
        c.sample(point.0, point.1, point.2)
    );
}

#[test]
fn octaves_are_continuous_and_bounded() {
    let noise = Octaves::new(7, 8, 0);
    let limit: f64 = (0..8).map(|j| 2f64.powi(j)).sum::<f64>() * 1.5;
    for i in 0..500 {
        let x = f64::from(i) * 0.37;
        let value = noise.sample(x, 1.1, 2.2);
        assert!(value.abs() < limit);
        assert!((noise.sample(x + 1e-6, 1.1, 2.2) - value).abs() < 1e-3);
    }
}
