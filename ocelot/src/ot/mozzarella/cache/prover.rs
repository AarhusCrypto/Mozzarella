#[derive(Clone)]
pub struct CachedProver<T: Copy + Clone> {
    u: Vec<T>, // cache
    w: Vec<T>, // cache
}

impl<T: Copy + Clone> CachedProver<T> {
    pub fn init(u: Vec<T>, w: Vec<T>) -> Self {
        Self { u, w }
    }

    pub fn get(&mut self, amount: usize) -> (Vec<T>, Vec<T>) {
        (
            self.u.split_off(self.u.len() - amount),
            self.w.split_off(self.w.len() - amount),
        )
    }

    pub fn pop(&mut self) -> (T, T) {
        let u = self.u.pop();
        let w = self.w.pop();
        (u.unwrap(), w.unwrap())
    }

    pub fn capacity(&self) -> usize {
        self.u.len()
    }

    pub fn append<I1: Iterator<Item = T>, I2: Iterator<Item = T>>(&mut self, u: I1, w: I2) {
        self.u.extend(u);
        self.w.extend(w);
        debug_assert_eq!(self.u.len(), self.w.len());
    }
}
