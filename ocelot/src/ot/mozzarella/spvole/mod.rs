pub mod prover;
pub mod verifier;

#[cfg(test)]
mod tests {
    use super::{prover::BatchedProver, verifier::BatchedVerifier};
    use crate::ot::mozzarella::cache::cacheinit::GenCache;
    use rand::{rngs::OsRng, Rng};
    use scuttlebutt::{ring::R64, unix_channel_pair, Block};
    use std::thread::spawn;

    #[test]
    fn test_batched_sp_vole() {
        const TEST_REPETITIONS: usize = 10;

        const NUM_SP_VOLES: usize = 16;
        const LOG_SINGLE_OUTPUT_SIZE: usize = 8;
        const SINGLE_OUTPUT_SIZE: usize = 1 << LOG_SINGLE_OUTPUT_SIZE;
        const OUTPUT_SIZE: usize = SINGLE_OUTPUT_SIZE * NUM_SP_VOLES;
        const CACHE_SIZE: usize = 2 * NUM_SP_VOLES * TEST_REPETITIONS;
        let mut rng = OsRng;

        for _ in 0..TEST_REPETITIONS {
            let fixed_key: Block = rng.gen();
            let delta: R64 = R64(fixed_key.extract_0_u64());
            let (mut cached_prover, mut cached_verifier) =
                GenCache::new::<R64, _, 0, CACHE_SIZE>(&mut rng, delta);
            let all_base_vole_p = cached_prover.get(CACHE_SIZE);
            let all_base_vole_v = cached_verifier.get(CACHE_SIZE);
            assert_eq!(all_base_vole_p.0.len(), CACHE_SIZE);
            assert_eq!(all_base_vole_p.1.len(), CACHE_SIZE);
            assert_eq!(all_base_vole_v.len(), CACHE_SIZE);

            let mut sp_prover = BatchedProver::<R64>::new(NUM_SP_VOLES, LOG_SINGLE_OUTPUT_SIZE);
            let mut sp_verifier = BatchedVerifier::<R64>::new(NUM_SP_VOLES, LOG_SINGLE_OUTPUT_SIZE);
            let (mut channel_p, mut channel_v) = unix_channel_pair();
            let mut alphas = [0usize; NUM_SP_VOLES];
            let mut out_u = [R64::default(); OUTPUT_SIZE];
            let mut out_w = [R64::default(); OUTPUT_SIZE];
            let mut out_v = [R64::default(); OUTPUT_SIZE];

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
                        assert_eq!(out_u[base + i], R64::default());
                        assert_eq!(out_w[base + i], out_v[base + i]);
                    }
                }
            }
        }
    }
}
