use super::*;

#[test]
fn two_presses_in_window_fire() {
    let mut tap = DoubleTap::new(0.3);
    assert!(!tap.press(1.0));
    assert!(tap.press(1.25));
}

#[test]
fn presses_outside_window_do_not_fire() {
    let mut tap = DoubleTap::new(0.3);
    assert!(!tap.press(1.0));
    assert!(!tap.press(1.4));
    // Опоздавшее нажатие начинает новую пару.
    assert!(tap.press(1.6));
}

#[test]
fn third_quick_press_starts_new_pair() {
    let mut tap = DoubleTap::new(0.3);
    assert!(!tap.press(1.0));
    assert!(tap.press(1.1));
    assert!(!tap.press(1.2));
    assert!(tap.press(1.3));
}
