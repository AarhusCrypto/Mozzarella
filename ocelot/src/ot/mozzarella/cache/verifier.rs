use scuttlebutt::ring::R64;

pub struct CachedVerifier {
    v: Vec<R64>, // cache
    delta: R64,
}

impl CachedVerifier {

    pub fn init(v: Vec<R64>, delta: R64) -> Self {
        Self {
            v,
            delta,
        }
    }

    pub fn append<I1: Iterator<Item=R64>>(&mut self, v: I1) {
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