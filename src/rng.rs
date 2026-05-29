pub struct OfferRng {
    state: u64,
}

impl OfferRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0xA2E3_4F9B_781C_D012,
        }
    }

    pub fn next_index(&mut self, upper_bound: usize) -> usize {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state as usize) % upper_bound
    }
}
