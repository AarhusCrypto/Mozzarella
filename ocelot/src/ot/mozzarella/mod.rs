use crate::ot::mozzarella::{prover::Prover, verifier::Verifier};
use lazy_static::lazy_static;

use crate::ot::mozzarella::lpn::LLCode;
use scuttlebutt::Block;

pub mod cache;
pub mod ggm;
pub mod lpn;
mod prover;
pub mod spvole;
pub mod utils;
mod verifier;

pub type MozzarellaProver<'a> = Prover<'a>;
pub type MozzarellaVerifier<'a> = Verifier<'a>;

pub const fn reg_vole_required(k: usize, t: usize) -> usize {
    k + (t * 2)
}

const CODE_D: usize = 10;

// benchmarking parameters
//pub const REG_MAIN_K: usize = 589_760; // TODO: remove this eventually, when cache works
pub const REG_MAIN_K: usize = 400; // TODO: remove this eventually, when cache works
//pub const REG_MAIN_T: usize = 1_319; // TODO: remove this eventually, when cache works
pub const REG_MAIN_T: usize = 1; // TODO: remove this eventually, when cache works
//const REG_MAIN_N: usize = 10_805_248;
const REG_MAIN_N: usize = 8192;
pub const REG_MAIN_LOG_SPLEN: usize = 13;
pub const REG_MAIN_SPLEN: usize = 1 << REG_MAIN_LOG_SPLEN;

// testing parameters
// pub const REG_MAIN_K: usize = 500; // TODO: remove this eventually, when cache works
// pub const REG_MAIN_T: usize = 20; // TODO: remove this eventually, when cache works
// pub const REG_MAIN_N: usize = 10240;
// pub const REG_MAIN_LOG_SPLEN: usize = 9;
// pub const REG_MAIN_SPLEN: usize = 1 << REG_MAIN_LOG_SPLEN;

pub const REG_MAIN_VOLE: usize = reg_vole_required(REG_MAIN_K, REG_MAIN_T);

lazy_static! {
    pub static ref REG_MAIN_CODE: LLCode =
        LLCode::from_seed(REG_MAIN_K, REG_MAIN_N, CODE_D, Block::default());
    static ref REG_TEST_CODE: LLCode = LLCode::from_seed(10, 64, 4, Block::default());
}

pub fn init_lpn() {
    lazy_static::initialize(&REG_MAIN_CODE);
}

#[cfg(test)]
mod tests {
    use super::{MozzarellaProver, MozzarellaVerifier, REG_TEST_CODE};
    use crate::ot::mozzarella::cache::cacheinit::GenCache;
    use rand::{rngs::OsRng, Rng};
    use scuttlebutt::{ring::R64, unix_channel_pair, Block};
    use std::thread::spawn;

    #[test]
    fn test_vole_extension() {
        const TEST_REPETITIONS: usize = 10;

        const LOG_SINGLE_SP_OUTPUT_SIZE: usize = 4;
        const SINGLE_SP_OUTPUT_SIZE: usize = 1 << LOG_SINGLE_SP_OUTPUT_SIZE;
        const BASE_VOLE_LEN: usize = 10;
        const CACHE_SIZE: usize = (BASE_VOLE_LEN + 2 * NUM_SP_VOLES) * TEST_REPETITIONS;
        const NUM_SP_VOLES: usize = 4;
        const OUTPUT_SIZE: usize = NUM_SP_VOLES * SINGLE_SP_OUTPUT_SIZE;
        assert_eq!(OUTPUT_SIZE, 64);
        lazy_static::initialize(&REG_TEST_CODE);
        let mut rng = OsRng;

        for _ in 0..TEST_REPETITIONS {
            let fixed_key: Block = rng.gen();
            let delta: R64 = R64(fixed_key.extract_0_u64());
            let (mut cached_prover, mut cached_verifier) =
                GenCache::new::<_, 0, CACHE_SIZE>(&mut rng, delta);
            let all_base_vole_p = cached_prover.get(CACHE_SIZE);
            let all_base_vole_v = cached_verifier.get(CACHE_SIZE);
            assert_eq!(all_base_vole_p.0.len(), CACHE_SIZE);
            assert_eq!(all_base_vole_p.1.len(), CACHE_SIZE);
            assert_eq!(all_base_vole_v.len(), CACHE_SIZE);

            let mut prover = MozzarellaProver::new(
                cached_prover,
                &REG_TEST_CODE,
                BASE_VOLE_LEN,
                NUM_SP_VOLES,
                LOG_SINGLE_SP_OUTPUT_SIZE,
            );
            let mut verifier = MozzarellaVerifier::new(
                cached_verifier,
                &REG_TEST_CODE,
                BASE_VOLE_LEN,
                NUM_SP_VOLES,
                LOG_SINGLE_SP_OUTPUT_SIZE,
            );
            let (mut channel_p, mut channel_v) = unix_channel_pair();

            let prover_thread = spawn(move || {
                prover.init(&mut channel_p).unwrap();
                prover.extend(&mut channel_p).unwrap()
            });

            let verifier_thread = spawn(move || {
                verifier.init(&mut channel_v, &fixed_key.into()).unwrap();
                verifier.extend(&mut channel_v).unwrap()
            });

            let (out_u, out_w) = prover_thread.join().unwrap();
            let out_v = verifier_thread.join().unwrap();

            assert_eq!(out_u.len(), OUTPUT_SIZE);
            assert_eq!(out_w.len(), OUTPUT_SIZE);
            assert_eq!(out_v.len(), OUTPUT_SIZE);
            for i in 0..OUTPUT_SIZE {
                assert_eq!(out_w[i], delta * out_u[i] + out_v[i]);
            }
        }
    }
}
