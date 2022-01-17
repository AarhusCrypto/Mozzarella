#[derive(Clone)]
pub struct CachedVerifier<T> {
    v: Vec<T>, // cache
}

impl<T: Copy + Clone> CachedVerifier<T> {
    pub fn init(v: Vec<T>) -> Self {
        Self { v }
    }

    pub fn append<I1: Iterator<Item = T>>(&mut self, v: I1) {
        self.v.extend(v);
    }

    pub fn pop(&mut self) -> T {
        self.v.pop().unwrap()
    }

    pub fn get(&mut self, amount: usize) -> Vec<T> {
        self.v[..amount].to_vec()
    }

    pub fn capacity(&self) -> usize {
        self.v.len()
    }
}
