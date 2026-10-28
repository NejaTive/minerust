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
mod tests;
