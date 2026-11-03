use crate::trees::splitmix64;

pub struct Stream(u64);

impl Stream {
    pub fn new(state: u64) -> Self {
        Self(state)
    }

    pub fn below(&mut self, bound: u32) -> u32 {
        self.0 = self.0.wrapping_add(1);
        (splitmix64(self.0) >> 32) as u32 % bound
    }

    pub fn taper(&mut self) -> u32 {
        let bound = self.below(3) + 1;
        self.below(bound)
    }

    pub fn flip(&mut self) -> bool {
        self.below(2) == 1
    }

    pub fn chance(&mut self, numerator: u32, denominator: u32) -> bool {
        self.below(denominator) < numerator
    }

    pub fn spread(&mut self, reach: u32) -> i32 {
        self.below(reach) as i32 - self.below(reach) as i32
    }
}
