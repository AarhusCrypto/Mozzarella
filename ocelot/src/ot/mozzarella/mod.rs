use crate::ot::mozzarella::{
    prover::{Prover, ProverStats},
    verifier::{Verifier, VerifierStats},
};
use lazy_static::lazy_static;

use crate::ot::mozzarella::lpn::LLCode;
use scuttlebutt::{ring::R64, Block};

pub mod cache;
pub mod ggm;
pub mod lpn;
mod prover;
pub mod spvole;
pub mod utils;
mod verifier;

pub type MozzarellaProver<'a, RingT> = Prover<'a, RingT>;
pub type MozzarellaVerifier<'a, RingT> = Verifier<'a, RingT>;
pub type MozzarellaProverStats = ProverStats;
pub type MozzarellaVerifierStats = VerifierStats;

pub const fn reg_vole_required(k: usize, t: usize) -> usize {
    k + (t * 2)
}

pub const CODE_D: usize = 10;

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
    pub static ref REG_MAIN_CODE: LLCode<R64> =
        LLCode::<R64>::from_seed(REG_MAIN_K, REG_MAIN_N, CODE_D, Block::default());
    static ref REG_TEST_CODE: LLCode<R64> = LLCode::<R64>::from_seed(10, 64, 4, Block::default());
}

pub fn init_lpn() {
    lazy_static::initialize(&REG_MAIN_CODE);
}

#[cfg(test)]
mod tests {
    use super::{LLCode, MozzarellaProver, MozzarellaVerifier, CODE_D};
    use crate::ot::mozzarella::cache::cacheinit::GenCache;
    use rand::{
        distributions::{Distribution, Standard},
        rngs::OsRng,
        Rng,
    };
    use scuttlebutt::{
        channel::{Receivable, Sendable},
        ring::{z2r, NewRing, R64},
        unix_channel_pair,
        Block,
    };
    use std::{sync::Arc, thread::spawn};

    fn test_vole_extension<RingT, const NIGHTLY: bool>()
    where
        RingT: NewRing + Receivable,
        Standard: Distribution<RingT>,
        for<'a> &'a RingT: Sendable,
    {
        const TEST_REPETITIONS: usize = 10;

        const LOG_SINGLE_SP_OUTPUT_SIZE: usize = 4;
        const SINGLE_SP_OUTPUT_SIZE: usize = 1 << LOG_SINGLE_SP_OUTPUT_SIZE;
        const BASE_VOLE_LEN: usize = 10;
        const CACHE_SIZE: usize = (BASE_VOLE_LEN + 2 * NUM_SP_VOLES) * TEST_REPETITIONS;
        const NUM_SP_VOLES: usize = 4;
        const OUTPUT_SIZE: usize = NUM_SP_VOLES * SINGLE_SP_OUTPUT_SIZE;
        assert_eq!(OUTPUT_SIZE, 64);

        let code = Arc::new(LLCode::<RingT>::from_seed(
            BASE_VOLE_LEN,
            OUTPUT_SIZE,
            CODE_D,
            Block::default(),
        ));
        let mut rng = OsRng;

        for _ in 0..TEST_REPETITIONS {
            let delta = rng.gen::<RingT>().reduce_to::<40>();
            let (mut cached_prover, mut cached_verifier) =
                GenCache::new::<RingT, _, 0, CACHE_SIZE>(&mut rng, delta);
            let all_base_vole_p = cached_prover.get(CACHE_SIZE);
            let all_base_vole_v = cached_verifier.get(CACHE_SIZE);
            assert_eq!(all_base_vole_p.0.len(), CACHE_SIZE);
            assert_eq!(all_base_vole_p.1.len(), CACHE_SIZE);
            assert_eq!(all_base_vole_v.len(), CACHE_SIZE);

            let (mut channel_p, mut channel_v) = unix_channel_pair();
            let code_p = code.clone();
            let code_v = code.clone();

            let prover_thread = spawn(move || {
                let mut prover = MozzarellaProver::<RingT>::new(
                    cached_prover,
                    &code_p,
                    BASE_VOLE_LEN,
                    NUM_SP_VOLES,
                    SINGLE_SP_OUTPUT_SIZE,
                    NIGHTLY,
                );
                prover.init(&mut channel_p).unwrap();
                prover.extend(&mut channel_p).unwrap()
            });

            let verifier_thread = spawn(move || {
                let mut verifier = MozzarellaVerifier::<RingT>::new(
                    cached_verifier,
                    &code_v,
                    BASE_VOLE_LEN,
                    NUM_SP_VOLES,
                    SINGLE_SP_OUTPUT_SIZE,
                    NIGHTLY,
                );
                verifier.init(&mut channel_v, delta).unwrap();
                verifier.extend(&mut channel_v).unwrap()
            });

            let (out_u, out_w) = prover_thread.join().unwrap();
            let out_v = verifier_thread.join().unwrap();

            assert!(out_u.iter().all(|x| x.is_reduced()));
            assert!(out_w.iter().all(|x| x.is_reduced()));
            assert!(out_v.iter().all(|x| x.is_reduced()));
            assert_eq!(out_u.len(), OUTPUT_SIZE);
            assert_eq!(out_w.len(), OUTPUT_SIZE);
            assert_eq!(out_v.len(), OUTPUT_SIZE);
            for i in 0..OUTPUT_SIZE {
                assert_eq!(out_w[i], delta * out_u[i] + out_v[i]);
            }
        }
    }

    #[test]
    fn test_vole_extension_r64() {
        test_vole_extension::<R64, false>();
    }

    #[test]
    fn test_vole_extension_r104() {
        test_vole_extension::<z2r::R104, false>();
    }

    #[test]
    fn test_vole_extension_r144() {
        test_vole_extension::<z2r::R144, false>();
    }
}
