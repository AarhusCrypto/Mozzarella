use scuttlebutt::Block;


pub struct BiasedGen {
    x: u64,
    s: u64,
    limit: u64,
}

impl BiasedGen {
    pub fn new(seed: Block, limit: u64) -> BiasedGen {
        BiasedGen {
            x: seed.extract_0_u64(),
            s: seed.extract_0_u64(),
            limit,
        }
    }

    pub fn next(&mut self) -> u64 {
        let out = self.s % self.limit;
        self.s = self.s.overflowing_mul(self.x).0;
        out
    }
}
