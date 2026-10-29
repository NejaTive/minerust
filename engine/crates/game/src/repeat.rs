pub struct Repeat {
    interval: f32,
    held: bool,
    pending: bool,
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

    pub fn set(&mut self, pressed: bool) {
        if pressed && !self.held {
            self.pending = true;
        }
        self.held = pressed;
    }

    pub fn reset(&mut self) {
        self.held = false;
        self.pending = false;
    }

    pub fn fire(&mut self, time: f32) -> bool {
        if self.pending {
            self.pending = false;
            self.next = time + self.interval;
            return true;
        }
        if self.held && time >= self.next {
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
