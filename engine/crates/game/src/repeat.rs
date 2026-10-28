/// Действие по удержанию кнопки: первое срабатывание — сразу при нажатии, дальше
/// раз в `interval` секунд, пока кнопка зажата. Время — секунды из `Engine::render`.
pub struct Repeat {
    interval: f32,
    held: bool,
    /// Нажатие ещё не отработано. Держится и после отпускания: иначе клик, который
    /// начался и закончился между двумя кадрами, потерялся бы.
    pending: bool,
    /// Время следующего повтора при удержании.
    next: f32,
}

impl Repeat {
    pub fn new(interval: f32) -> Self {
        Self {
            interval,
            held: false,
            pending: false,
            next: 0.0,
        }
    }

    /// Нажатие или отпускание. Повторное нажатие без отпускания ничего не меняет.
    pub fn set(&mut self, pressed: bool) {
        if pressed && !self.held {
            self.pending = true;
        }
        self.held = pressed;
    }

    /// Отпускает кнопку и забывает неотработанное нажатие (потеря pointer lock).
    pub fn reset(&mut self) {
        self.held = false;
        self.pending = false;
    }

    /// Пора ли действовать в момент `time`. Вызывать раз в кадр: срабатывание
    /// засчитывается, даже если действовать не на что (нет блока под прицелом).
    pub fn fire(&mut self, time: f32) -> bool {
        if self.pending {
            self.pending = false;
            self.next = time + self.interval;
            return true;
        }
        if self.held && time >= self.next {
            // От прошлого срабатывания, а не от кадра: иначе каждый повтор опаздывал бы
            // на долю кадра, и темп был бы ниже заданного. После долгой паузы кадров
            // догонять пропущенное не нужно.
            self.next += self.interval;
            if self.next <= time {
                self.next = time + self.interval;
            }
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Времена срабатываний при вызове `fire` с шагом `dt` от `from` до `to`.
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
        // 60 Гц: интервал не кратен кадру, но за 2 с должно быть ровно 1 + 8 срабатываний.
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
}
