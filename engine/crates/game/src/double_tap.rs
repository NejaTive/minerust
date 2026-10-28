/// Двойное нажатие: второе нажатие не позже чем через `window` секунд после первого.
/// Время — секунды из `Engine::render`.
pub struct DoubleTap {
    window: f32,
    /// Время первого нажатия пары; `None` — пара не начата.
    first: Option<f32>,
}

impl DoubleTap {
    pub fn new(window: f32) -> Self {
        Self {
            window,
            first: None,
        }
    }

    /// Нажатие в момент `time` (с). `true`, если это второе нажатие в окне; после
    /// срабатывания пара сбрасывается, и третье нажатие подряд снова считается первым.
    pub fn press(&mut self, time: f32) -> bool {
        match self.first {
            Some(first) if time - first <= self.window => {
                self.first = None;
                true
            }
            _ => {
                self.first = Some(time);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
}
