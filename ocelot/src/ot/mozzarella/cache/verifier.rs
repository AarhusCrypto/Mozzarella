use scuttlebutt::ring::R64;

pub struct CachedVerifier {
    v: Vec<R64>, // cache
}

impl CachedVerifier {
    pub fn init(v: Vec<R64>) -> Self {
        Self { v }
    }

    pub fn append<I1: Iterator<Item = R64>>(&mut self, v: I1) {
        self.v.extend(v);
    }

    pub fn pop(&mut self) -> R64 {
        self.v.pop().unwrap()
    }

    pub fn get(&mut self, amount: usize) -> Vec<R64> {
        self.v[..amount].to_vec()
    }

    pub fn capacity(&self) -> usize {
        self.v.len()
    }
}
