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

    pub fn skip(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }

    pub fn fork(&mut self) -> Stream {
        let high = u64::from(self.below(u32::MAX));
        Stream::new(high << 32 | u64::from(self.below(u32::MAX)))
    }

    pub fn taper(&mut self) -> u32 {
        let bound = self.below(3) + 1;
        self.below(bound)
    }

    pub fn fraction(&mut self) -> f64 {
        f64::from(self.below(1 << 24)) / f64::from(1 << 24)
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
