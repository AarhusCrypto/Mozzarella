use scuttlebutt::ring::R64;

/// A collection of correlated OT outputs
pub struct CachedProver {
    u: Vec<R64>, // cache
    w: Vec<R64>, // cache
}

impl CachedProver {
    pub fn init(u: Vec<R64>, w: Vec<R64>) -> Self {
        Self { u, w }
    }

    pub fn get(&mut self, amount: usize) -> (Vec<R64>, Vec<R64>) {
        (self.u[..amount].to_vec(), self.w[..amount].to_vec())
    }

    pub fn pop(&mut self) -> (R64, R64) {
        let u = self.u.pop();
        let w = self.w.pop();
        (u.unwrap(), w.unwrap())
    }

    pub fn capacity(&self) -> usize {
        self.u.len()
    }

    pub fn append<I1: Iterator<Item = R64>, I2: Iterator<Item = R64>>(&mut self, u: I1, w: I2) {
        self.u.extend(u);
        self.w.extend(w);
        debug_assert_eq!(self.u.len(), self.w.len());
    }
}
