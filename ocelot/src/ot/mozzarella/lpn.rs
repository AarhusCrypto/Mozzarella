use rand::{CryptoRng, Rng, SeedableRng};

use crate::ot::mozzarella::utils::gen_column;
use rayon::prelude::*;
use scuttlebutt::{ring::R64, AesRng, Block};

// Z64 Local Linear Code with parameter D
pub struct LLCode<const ROWS: usize, const COLS: usize, const D: usize> {
    indices: Vec<[(usize, R64); D]>,
}

// columns have length of rows
impl<const ROWS: usize, const COLS: usize, const D: usize> LLCode<ROWS, COLS, D> {
    pub fn from_seed(seed: Block) -> Self {
        let mut rng = AesRng::from_seed(seed);
        Self::gen(&mut rng)
    }

    pub fn gen<R: Rng + CryptoRng>(rng: &mut R) -> Self {
        let max_val = ((0 as u64).overflowing_sub(1).0) as usize;
        let mut code = LLCode {
            indices: Vec::with_capacity(COLS),
        };
        for _ in 0..COLS {
            code.indices.push(gen_column::<_, D>(rng, ROWS, max_val));
        }

        println!("COLS:\t {}", COLS);
        code.indices.sort(); // TODO: test this - sorting the rows, seems to improve cache locality
        code
    }

    // TODO: Can likely be made more efficient somehow?
    pub fn mul(&self, v: &[R64]) -> Vec<R64> {
        (self.indices.par_iter().map(|col| {
            let mut cord: R64 = R64::default();
            for i in col.iter().copied() {
                cord += i.1 * v[i.0];
            }
            cord
        }))
        .collect()
    }

    // takes the indices of the code (A) and adds them to elements of a.
    pub fn mul_add(&self, v: &[R64], a: &[R64]) -> Vec<R64> {
        (self.indices.par_iter().enumerate().map(|(j, col)| {
            let mut cord: R64 = R64::default();
            for i in col.iter().copied() {
                cord += i.1 * v[i.0];
            }
            cord + a[j]
        }))
        .collect()
    }
}
