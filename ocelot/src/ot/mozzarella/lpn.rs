use rand::{CryptoRng, Rng, SeedableRng};

use scuttlebutt::{AesRng, Block};
use std::ops::{BitXorAssign, MulAssign};
use scuttlebutt::ring::R64;
use crate::ot::mozzarella::utils::unique_random_array;
use crate::ot::mozzarella::utils::gen_column;


// Z64 Local Linear Code with parameter D
pub struct LLCode<const ROWS: usize, const COLS: usize, const D: usize> {
    indices: Vec<[(usize, R64); D]>,
}



// TODO: I'm an idiot; it should contain 10 per COLUMN that are non-zero, not rows!!
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
        //println!("COLS: {}, ROWS: {}", COLS, ROWS);
        for _ in 0..COLS {
            code.indices.push(gen_column::<_, D>(rng, ROWS, max_val));
            //code.indices.push(unique_random_array(rng, ROWS));
        }
        /*for j in &code.indices {
            for i in j {
                println!("CODE.INDICIES:\t {}", i.1);
            }
        }*/
        //println!("length: {}", code.indices[0].len());
        code.indices.sort(); // sorting the rows, seems to improve cache locality
        code
    }

    // TODO: Can likely be made more efficient somehow?
    pub fn mul(&self, v: &[R64]) -> Vec<R64> {
        let mut r: Vec<R64> = Vec::with_capacity(COLS);
        for col in self.indices.iter() {
            let mut cord: R64 = R64::default();
            let mut tmp: R64 = R64::default();
            //let mut column: Vec<u64> = col.iter().map(|x| x.1.0).collect();
            //println!("COLUMN:\t {:?}", column);
            for i in col.iter().copied() {
                tmp = i.1;
                //println!("MULTIPLYING (index {}):\t {} x {}", i.0, i.1, v[i.0]);
                tmp *= v[i.0];
                cord += tmp;
            }
            //println!("CORD:\t {}",cord);
            r.push(cord);
        }
        //println!("LENGTH_OF_r:\t {}", &r.len());
        r
    }

    // TODO: this is not working! Currently it only multiplies, but it does not add the vector correctly
    // takes the indices of the code (A) and adds them to elements of a. Probably corresponds to first multiplying v with A and then adding it!
    pub fn mul_add(&self, v: &[R64], a: &[R64]) -> Vec<R64> {
        let mut out: Vec<R64> = Vec::new();
        for (j, col) in self.indices.iter().enumerate() {
            let mut tmp: R64 = R64::default();
            let mut cord: R64 = R64::default();
            for i in col.iter().copied() {
                tmp = i.1;
                tmp *= v[i.0];
                //println!("MULTIPLYING (index {}):\t {} x {}", i.0, i.1, v[i.0]);
                cord += tmp;
            }

            //println!("RESULT CORD (index {}):\t {}", j, cord);
            cord += a[j]; // TODO: works?
            //println!("ADDING (index {}):\t {} = {}", j, a[j], cord);
            out.push(cord);
        }
        //println!("LENGTH_PROVER_OUT:\t {}", &out.len());
        out
    }
}

// none of these work currently
#[cfg(test)]
mod tests {
    use super::*;

    const ROWS: usize = 30;
    const COLS: usize = 50;
    const D: usize = 10;

    use std::convert::TryInto;

    use rand::{rngs::StdRng, Rng, SeedableRng};

    #[test]
    fn test_bool_linear() {
        let mut rng = StdRng::seed_from_u64(0x5322_FA41_6AB1_521A);
        for _ in 0..10 {
            let code: LLCode<ROWS, COLS, D> = LLCode::gen(&mut rng);

            let a: Vec<bool> = (0..ROWS).map(|_| rng.gen()).collect();
            let b: Vec<bool> = (0..ROWS).map(|_| rng.gen()).collect();
            let ab: Vec<bool> = (0..ROWS).map(|i| a[i] ^ b[i]).collect();

            let a_c = code.mul((&a[..]).try_into().unwrap());
            let b_c = code.mul((&b[..]).try_into().unwrap());
            let ab_c = code.mul((&ab[..]).try_into().unwrap());
            let a_c_b_c: Vec<bool> = (0..COLS).map(|i| a_c[i] ^ b_c[i]).collect();
            assert_eq!(a_c_b_c, ab_c);
        }
    }

    #[test]
    fn test_block_linear() {
        let mut rng = StdRng::seed_from_u64(0x5322_FA41_6AB1_521A);
        for _ in 0..10 {
            let code: LLCode<ROWS, COLS, D> = LLCode::gen(&mut rng);

            let a: Vec<Block> = (0..ROWS).map(|_| rng.gen()).collect();
            let b: Vec<Block> = (0..ROWS).map(|_| rng.gen()).collect();
            let ab: Vec<Block> = (0..ROWS).map(|i| a[i] ^ b[i]).collect();

            let a_c = code.mul((&a[..]).try_into().unwrap());
            let b_c = code.mul((&b[..]).try_into().unwrap());
            let ab_c = code.mul((&ab[..]).try_into().unwrap());
            let a_c_b_c: Vec<Block> = (0..COLS).map(|i| a_c[i] ^ b_c[i]).collect();
            assert_eq!(a_c_b_c, ab_c);
        }
    }
}
