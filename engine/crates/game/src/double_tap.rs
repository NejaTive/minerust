pub struct DoubleTap {
    window: f32,
    first: Option<f32>,
}

impl DoubleTap {
    pub fn new(window: f32) -> Self {
        Self {
            window,
            first: None,
        }
    }

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
mod tests;
