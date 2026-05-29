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
        (self.next_u64() as usize) % upper_bound
    }

    /// Uniform float in `[0, 1)`, used for chance rolls like crits.
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
}
