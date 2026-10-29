use super::*;

fn fires(repeat: &mut Repeat, from: f32, to: f32, dt: f32) -> Vec<f32> {
    // Время кадра — от номера, а не суммой: сумма f32 копит ошибку.
    (0..)
        .map(|i| from + i as f32 * dt)
        .take_while(|&t| t < to)
        .filter(|&t| repeat.fire(t))
        .collect()
}

#[test]
fn fires_at_once_then_every_interval() {
    let mut repeat = Repeat::new(0.25);
    assert!(!repeat.fire(0.0));
    repeat.set(true);
    let times = fires(&mut repeat, 10.0, 11.01, 0.01);
    assert_eq!(times.len(), 5, "{times:?}");
    assert_eq!(times[0], 10.0);
    for pair in times.windows(2) {
        assert!((pair[1] - pair[0] - 0.25).abs() < 0.011, "{times:?}");
    }
}

#[test]
fn keeps_pace_at_low_frame_rate() {
    let mut repeat = Repeat::new(0.25);
    repeat.set(true);
    assert_eq!(fires(&mut repeat, 0.0, 2.0 + 1e-3, 1.0 / 60.0).len(), 9);
}

#[test]
fn stops_on_release() {
    let mut repeat = Repeat::new(0.25);
    repeat.set(true);
    assert!(repeat.fire(0.0));
    repeat.set(false);
    assert!(fires(&mut repeat, 0.0, 2.0, 0.01).is_empty());
}

#[test]
fn click_between_frames_fires_once() {
    let mut repeat = Repeat::new(0.25);
    repeat.set(true);
    repeat.set(false);
    assert!(repeat.fire(5.0));
    assert!(!repeat.fire(6.0));
}

#[test]
fn repeated_press_does_not_restart() {
    let mut repeat = Repeat::new(0.25);
    repeat.set(true);
    assert!(repeat.fire(0.0));
    repeat.set(true);
    assert!(!repeat.fire(0.1));
}

#[test]
fn reset_drops_pending_click() {
    let mut repeat = Repeat::new(0.25);
    repeat.set(true);
    repeat.reset();
    assert!(!repeat.fire(0.0));
}
