pub mod prover;
pub mod verifier;

#[cfg(test)]
mod tests {
    use super::{prover::BatchedProver, verifier::BatchedVerifier};
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
    };
    use std::thread::spawn;

    fn test_batched_sp_vole<RingT, const NIGHTLY: bool>()
    where
        RingT: NewRing + Receivable,
        Standard: Distribution<RingT>,
        for<'a> &'a RingT: Sendable,
    {
        const TEST_REPETITIONS: usize = 10;

        const NUM_SP_VOLES: usize = 16;
        const LOG_SINGLE_OUTPUT_SIZE: usize = 8;
        const SINGLE_OUTPUT_SIZE: usize = 1 << LOG_SINGLE_OUTPUT_SIZE;
        const OUTPUT_SIZE: usize = SINGLE_OUTPUT_SIZE * NUM_SP_VOLES;
        const CACHE_SIZE: usize = 2 * NUM_SP_VOLES * TEST_REPETITIONS;
        let mut rng = OsRng;

        for _ in 0..TEST_REPETITIONS {
            let delta = rng.gen::<RingT>();
            let (mut cached_prover, mut cached_verifier) =
                GenCache::new::<RingT, _, 0, CACHE_SIZE>(&mut rng, delta);
            let all_base_vole_p = cached_prover.get(CACHE_SIZE);
            let all_base_vole_v = cached_verifier.get(CACHE_SIZE);
            assert_eq!(all_base_vole_p.0.len(), CACHE_SIZE);
            assert_eq!(all_base_vole_p.1.len(), CACHE_SIZE);
            assert_eq!(all_base_vole_v.len(), CACHE_SIZE);

            let mut sp_prover =
                BatchedProver::<RingT>::new(NUM_SP_VOLES, LOG_SINGLE_OUTPUT_SIZE, NIGHTLY);
            let mut sp_verifier =
                BatchedVerifier::<RingT>::new(NUM_SP_VOLES, LOG_SINGLE_OUTPUT_SIZE, NIGHTLY);
            let (mut channel_p, mut channel_v) = unix_channel_pair();
            let mut alphas = [0usize; NUM_SP_VOLES];
            let mut out_u = vec![RingT::default(); OUTPUT_SIZE];
            let mut out_w = vec![RingT::default(); OUTPUT_SIZE];
            let mut out_v = vec![RingT::default(); OUTPUT_SIZE];

            let prover_thread = spawn(move || {
                sp_prover.init(&mut channel_p).unwrap();
                sp_prover
                    .extend(
                        &mut channel_p,
                        &mut cached_prover,
                        &mut alphas,
                        &mut out_u,
                        &mut out_w,
                    )
                    .unwrap();
                (out_u, out_w, alphas)
            });

            let verifier_thread = spawn(move || {
                sp_verifier.init(&mut channel_v, delta).unwrap();
                sp_verifier
                    .extend(&mut channel_v, &mut cached_verifier, &mut out_v)
                    .unwrap();
                out_v
            });

            let (out_u, out_w, alphas) = prover_thread.join().unwrap();
            let out_v = verifier_thread.join().unwrap();

            for alpha in alphas {
                assert!(alpha < SINGLE_OUTPUT_SIZE);
            }
            for j in 0..NUM_SP_VOLES {
                let base = j * SINGLE_OUTPUT_SIZE;
                for i in 0..SINGLE_OUTPUT_SIZE {
                    if i == alphas[j] {
                        assert_eq!(out_w[base + i], delta * out_u[base + i] + out_v[base + i]);
                    } else {
                        assert_eq!(out_u[base + i], RingT::default());
                        assert_eq!(out_w[base + i], out_v[base + i]);
                    }
                }
            }
        }
    }

    #[test]
    fn test_batched_sp_vole_r64() {
        test_batched_sp_vole::<R64, false>();
    }

    #[test]
    fn test_batched_sp_vole_r104() {
        test_batched_sp_vole::<z2r::R104, false>();
    }

    #[test]
    fn test_batched_sp_vole_r144() {
        test_batched_sp_vole::<z2r::R144, false>();
    }
}
