pub mod prover;
pub mod verifier;

#[cfg(test)]
mod tests {
    use super::{
        prover::{Prover, SingleProver},
        verifier::{SingleVerifier, Verifier},
    };
    use crate::ot::{
        mozzarella::cache::cacheinit::GenCache,
        FixedKeyInitializer,
        KosDeltaReceiver,
        KosDeltaSender,
        Receiver as OtReceiver,
    };
    use rand::{rngs::OsRng, Rng};
    use scuttlebutt::{ring::R64, unix_channel_pair, Block};
    use std::{convert::TryInto, thread::spawn};

    #[test]
    fn test_single_sp_vole() {
        const TEST_REPETITIONS: usize = 10;

        const LOG_OUTPUT_SIZE: usize = 8;
        const OUTPUT_SIZE: usize = 1 << LOG_OUTPUT_SIZE;
        const CACHE_SIZE: usize = 2 * TEST_REPETITIONS;
        let mut rng = OsRng;

        for test_i in 0..TEST_REPETITIONS {
            let fixed_key: Block = rng.gen();
            let delta: R64 = R64(fixed_key.extract_0_u64());
            let (mut cached_prover, mut cached_verifier) =
                GenCache::new::<_, 0, CACHE_SIZE>(&mut rng, delta);
            let all_base_vole_p = cached_prover.get(CACHE_SIZE);
            let all_base_vole_v = cached_verifier.get(CACHE_SIZE);
            assert_eq!(all_base_vole_p.0.len(), CACHE_SIZE);
            assert_eq!(all_base_vole_p.1.len(), CACHE_SIZE);
            assert_eq!(all_base_vole_v.len(), CACHE_SIZE);

            let mut sp_prover = SingleProver::new(0, LOG_OUTPUT_SIZE);
            let mut sp_verifier = SingleVerifier::new(0, LOG_OUTPUT_SIZE);
            let (mut channel_p, mut channel_v) = unix_channel_pair();
            let mut out_u = [R64::default(); OUTPUT_SIZE];
            let mut out_w = [R64::default(); OUTPUT_SIZE];
            let mut out_v = [R64::default(); OUTPUT_SIZE];

            let prover_thread = spawn(move || {
                let mut rng = OsRng;
                let mut ot_receiver = KosDeltaReceiver::init(&mut channel_p, &mut rng).unwrap();
                let base_vole: (&[R64; 2], &[R64; 2]) = (
                    all_base_vole_p.0[2 * test_i..2 * (test_i + 1)]
                        .try_into()
                        .unwrap(),
                    all_base_vole_p.1[2 * test_i..2 * (test_i + 1)]
                        .try_into()
                        .unwrap(),
                );
                sp_prover
                    .extend(
                        &mut channel_p,
                        &mut ot_receiver,
                        &mut out_u,
                        &mut out_w,
                        base_vole,
                    )
                    .unwrap();
                sp_prover.get_alpha()
            });

            let verifier_thread = spawn(move || {
                let mut rng = OsRng;
                let mut ot_sender =
                    KosDeltaSender::init_fixed_key(&mut channel_v, fixed_key.into(), &mut rng)
                        .unwrap();
                sp_verifier.init(delta);
                let base_vole: &[R64; 2] = all_base_vole_v[2 * test_i..2 * (test_i + 1)]
                    .try_into()
                    .unwrap();
                sp_verifier
                    .extend(&mut channel_v, &mut ot_sender, &mut out_v, base_vole)
                    .unwrap();
            });

            let alpha = prover_thread.join().unwrap();
            verifier_thread.join().unwrap();

            assert!(alpha < OUTPUT_SIZE);
            for i in 0..OUTPUT_SIZE {
                if i == alpha {
                    assert_eq!(out_w[i], delta * out_u[i] + out_v[i]);
                } else {
                    assert_eq!(out_u[i], R64::default());
                    assert_eq!(out_w[i], out_v[i]);
                }
            }
        }
    }

    #[test]
    fn test_multiple_sp_vole() {
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
                GenCache::new::<_, 0, CACHE_SIZE>(&mut rng, delta);
            let all_base_vole_p = cached_prover.get(CACHE_SIZE);
            let all_base_vole_v = cached_verifier.get(CACHE_SIZE);
            assert_eq!(all_base_vole_p.0.len(), CACHE_SIZE);
            assert_eq!(all_base_vole_p.1.len(), CACHE_SIZE);
            assert_eq!(all_base_vole_v.len(), CACHE_SIZE);

            let mut sp_prover = Prover::new(NUM_SP_VOLES, LOG_SINGLE_OUTPUT_SIZE);
            let mut sp_verifier = Verifier::new(NUM_SP_VOLES, LOG_SINGLE_OUTPUT_SIZE);
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
            });

            let verifier_thread = spawn(move || {
                sp_verifier.init(&mut channel_v, &fixed_key.into()).unwrap();
                sp_verifier
                    .extend(&mut channel_v, &mut cached_verifier, &mut out_v)
                    .unwrap();
            });

            prover_thread.join().unwrap();
            verifier_thread.join().unwrap();

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

//     use super::*;
//     use std::thread::spawn;
//     use rand::{rngs::StdRng, Rng, SeedableRng};
//     use rand::rngs::OsRng;
//     use scuttlebutt::{Block, unix_channel_pair};
//     use scuttlebutt::ring::R64;
//     use crate::ot::{FixedKeyInitializer, KosDeltaReceiver, KosDeltaSender, Receiver};
//     use crate::ot::mozzarella::cache::cacheinit::GenCache;
//     use crate::ot::mozzarella::cache::verifier::CachedVerifier;
//     use crate::ot::mozzarella::{REG_MAIN_N, REG_MAIN_T, reg_vole_required};
//     use crate::ot::mozzarella::spvole::prover::Prover;
//     use crate::ot::mozzarella::spvole::verifier::Verifier;
//     use crate::ot::mozzarella::utils::random_array;
//
//
//     fn test_spvole_correlation<const H: usize, const N: usize>(num: usize) {
//         let mut root = StdRng::seed_from_u64(0x5367_FA32_72B1_8478);
//         const CACHE_SIZE: usize = REG_MAIN_N + (2 * REG_MAIN_T);
//
//         for _ in 0..10 {
//             // de-randomize the test
//             let mut rng1 = StdRng::seed_from_u64(root.gen());
//             let mut rng2 = StdRng::seed_from_u64(root.gen());
//
//             let fixed_key: Block = rng1.gen();
//             let delta: R64 = R64(fixed_key.extract_0_u64()); // fyfy, TODO
//
//
//             let (mut c1, mut c2) = unix_channel_pair();
//             // let T=50, so we have enough lol
//             let (mut cached_prover,mut cached_verifier) =
//                 GenCache::new::<_,0,CACHE_SIZE>(&mut rng2,delta);
//             let handle = spawn(move || {
//
//
//                 let mut kos18_send =
//                     KosDeltaSender::init_fixed_key(&mut c2, fixed_key.into(), &mut rng2).unwrap();
//
//                 let mut verifier: Verifier = Verifier::init(delta);
//                 let v = verifier.extend::<_, _, _, N, H>(
//                     &mut c2, &mut rng2, num, &mut kos18_send, &mut cached_verifier
//                 ).unwrap();
//
//                 for j in &v {
//                     for i in j {
//                         println!("v={}", i);
//                     }
//                 }
//
//                 println!("delta={}", delta);
//
//                 (delta, v)
//             });
//
//             let mut kos18_rec = KosDeltaReceiver::init(&mut c1, &mut rng1).unwrap();
//
//
//
//             let mut prover: Prover = Prover::init();
//
//             let mut alphas = [0usize; 10]; // just sample too many alphas ..
//             for e in alphas.iter_mut() {
//                 *e = rng1.gen::<usize>() % N;
//             }
//
//             let (mut w, mut u) = prover.extend::<_, _, _, N, H>(
//                 &mut c1,
//                 &mut rng1,num,
//                 &mut kos18_rec,
//                 &mut cached_prover,
//                 &alphas
//             ).unwrap();
//
//             for j in &w {
//                 for i in j {
//                     println!("w={}", i);
//                 }
//             }
//
//             for j in &u {
//                 for i in j {
//                     println!("u={}", i);
//                 }
//             }
//
//             let (delta, mut v) = handle.join().unwrap();
//
//             for i in 0..num {
//                 u[i][alphas[i]] *= delta;
//                 v[i][alphas[i]] += u[i][alphas[i]];
//             }
//
//             assert_eq!(v, w, "correlation not satisfied");
//         }
//     }
//
//     #[test]
//     fn test_spvole_correlation_h2() {
//         for i in vec![1, 2, 5, 10].into_iter() {
//             test_spvole_correlation::<2, 4>(i);
//         }
//     }
//
//     #[test]
//     fn test_spvole_correlation_h3() {
//         for i in vec![1, 2, 5, 10].into_iter() {
//             test_spvole_correlation::<3, 8>(i);
//         }
//     }
//
//     #[test]
//     fn test_spvole_correlation_h4() {
//         for i in vec![1, 2, 5, 10].into_iter() {
//             test_spvole_correlation::<4, 16>(i);
//         }
//     }
//
//     #[test]
//     fn test_spvole_correlation_h5() {
//         for i in vec![1, 2, 5, 10].into_iter() {
//             test_spvole_correlation::<5, 32>(i);
//         }
//     }
// }
