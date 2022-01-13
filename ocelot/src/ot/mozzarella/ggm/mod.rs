mod generator;
pub mod prover;
pub mod verifier;

#[cfg(test)]
mod tests {
    use super::{
        prover::{BatchedProver, Prover},
        verifier::{BatchedVerifier, Verifier},
    };
    use crate::ot::{
        FixedKeyInitializer,
        KosDeltaReceiver,
        KosDeltaSender,
        Receiver as OtReceiver,
    };
    use rand::{rngs::OsRng, Rng};
    use scuttlebutt::unix_channel_pair;
    use std::thread::spawn;

    #[test]
    fn test_ggm_tree() {
        const TEST_REPETITIONS: usize = 10;

        const TREE_HEIGHT: usize = 8;
        const OUTPUT_SIZE: usize = 1 << TREE_HEIGHT;

        for _ in 0..TEST_REPETITIONS {
            let (mut channel_p, mut channel_v) = unix_channel_pair();

            let mut ggm_prover = Prover::new(TREE_HEIGHT);
            let mut ggm_verifier = Verifier::new(TREE_HEIGHT);
            let mut rng = OsRng;
            let ot_key: [u8; 16] = rng.gen();
            let alpha = 142;

            let prover_thread = spawn(move || {
                let mut rng = OsRng;
                let mut ot_receiver = KosDeltaReceiver::init(&mut channel_p, &mut rng).unwrap();
                ggm_prover
                    .gen_eval(&mut channel_p, &mut ot_receiver, alpha)
                    .unwrap();
                ggm_prover
            });
            let verifier_thread = spawn(move || {
                let mut rng = OsRng;
                let mut ot_sender =
                    KosDeltaSender::init_fixed_key(&mut channel_v, ot_key, &mut rng).unwrap();
                ggm_verifier
                    .gen_tree(&mut channel_v, &mut ot_sender)
                    .unwrap();
                ggm_verifier
            });

            let ggm_verifier = verifier_thread.join().unwrap();
            let ggm_prover = prover_thread.join().unwrap();
            let prover_values = ggm_prover.get_output_blocks();
            let verifier_values = ggm_verifier.get_output_blocks();
            assert_eq!(prover_values.len(), OUTPUT_SIZE);
            assert_eq!(verifier_values.len(), OUTPUT_SIZE);
            for i in 0..OUTPUT_SIZE {
                if i == alpha {
                    continue;
                }
                assert_eq!(prover_values[i], verifier_values[i]);
            }
        }
    }

    #[test]
    fn test_batched_ggm_tree() {
        const NUM_INSTANCES: usize = 5;
        const TEST_REPETITIONS: usize = 10;

        const TREE_HEIGHT: usize = 8;
        const OUTPUT_SIZE: usize = 1 << TREE_HEIGHT;

        for _ in 0..TEST_REPETITIONS {
            let (mut channel_p, mut channel_v) = unix_channel_pair();

            let mut ggm_prover = BatchedProver::new(NUM_INSTANCES, TREE_HEIGHT);
            let mut ggm_verifier = BatchedVerifier::new(NUM_INSTANCES, TREE_HEIGHT);
            let mut rng = OsRng;
            let ot_key: [u8; 16] = rng.gen();
            let alpha_s: [usize; NUM_INSTANCES] = [142, 47, 0, OUTPUT_SIZE - 1, 1];

            let prover_thread = spawn(move || {
                let mut rng = OsRng;
                let mut ot_receiver = KosDeltaReceiver::init(&mut channel_p, &mut rng).unwrap();
                ggm_prover
                    .gen_eval(&mut channel_p, &mut ot_receiver, &alpha_s)
                    .unwrap();
                ggm_prover
            });
            let verifier_thread = spawn(move || {
                let mut rng = OsRng;
                let mut ot_sender =
                    KosDeltaSender::init_fixed_key(&mut channel_v, ot_key, &mut rng).unwrap();
                ggm_verifier
                    .gen_tree(&mut channel_v, &mut ot_sender)
                    .unwrap();
                ggm_verifier
            });

            let ggm_verifier = verifier_thread.join().unwrap();
            let ggm_prover = prover_thread.join().unwrap();
            let prover_values = ggm_prover.get_output_blocks();
            let verifier_values = ggm_verifier.get_output_blocks();
            assert_eq!(prover_values.len(), NUM_INSTANCES * OUTPUT_SIZE);
            assert_eq!(verifier_values.len(), NUM_INSTANCES * OUTPUT_SIZE);
            for tree_j in 0..NUM_INSTANCES {
                for i in 0..OUTPUT_SIZE {
                    if i == alpha_s[tree_j] {
                        continue;
                    }
                    assert_eq!(
                        prover_values[tree_j * OUTPUT_SIZE + i],
                        verifier_values[tree_j * OUTPUT_SIZE + i]
                    );
                }
            }
        }
    }
}
