pub mod prover;
pub mod verifier;


#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::spawn;
    use rand::{rngs::StdRng, Rng, SeedableRng};
    use rand::rngs::OsRng;
    use scuttlebutt::{Block, unix_channel_pair};
    use scuttlebutt::ring::R64;
    use crate::ot::{FixedKeyInitializer, KosDeltaReceiver, KosDeltaSender, Receiver};
    use crate::ot::mozzarella::cache::cacheinit::GenCache;
    use crate::ot::mozzarella::cache::verifier::CachedVerifier;
    use crate::ot::mozzarella::spvole::prover::Prover;
    use crate::ot::mozzarella::spvole::verifier::Verifier;
    use crate::ot::mozzarella::utils::random_array;


    fn test_spvole_correlation<const H: usize, const N: usize>(num: usize) {
        let mut root = StdRng::seed_from_u64(0x5367_FA32_72B1_8478);
        const CACHE_SIZE: usize = 50;

        for _ in 0..10 {
            // de-randomize the test
            let mut rng1 = StdRng::seed_from_u64(root.gen());
            let mut rng2 = StdRng::seed_from_u64(root.gen());

            let fixed_key: Block = rng1.gen();
            let delta: R64 = R64(fixed_key.extract_0_u64()); // fyfy, TODO


            let (mut c1, mut c2) = unix_channel_pair();
            // let T=50, so we have enough lol
            let (mut cached_prover,mut cached_verifier) =
                GenCache::new::<_,0,CACHE_SIZE>(&mut rng2,delta);
            let handle = spawn(move || {


                let mut kos18_send =
                    KosDeltaSender::init_fixed_key(&mut c2, fixed_key.into(), &mut rng2).unwrap();
                //cache
                //    .generate(&mut kos18, &mut c2, &mut rng1, H * num + CSP)
                //    .unwrap();
                let mut verifier: Verifier = Verifier::init(delta);
                let v = verifier.extend::<_, _, _, N, H>(
                    &mut c2, &mut rng2, num, &mut kos18_send, &mut cached_verifier
                ).unwrap();

                for j in &v {
                    for i in j {
                        println!("v={}", i);
                    }
                }

                println!("delta={}", delta);

                (delta, v)
            });

            let mut kos18_rec = KosDeltaReceiver::init(&mut c1, &mut rng1).unwrap();



            let mut prover: Prover = Prover::init();
            //( let out = recv.receive_random(&mut c1, &[true], &mut OsRng).unwrap();

            let mut alphas = [0usize; 10]; // just sample too many alphas ..
            for e in alphas.iter_mut() {
                *e = rng1.gen::<usize>() % N;
            }

            let (mut w, mut u) = prover.extend::<_, _, _, N, H>(
                &mut c1,
                &mut rng1,num,
                &mut kos18_rec,
                &mut cached_prover,
                &alphas
            ).unwrap();

            for j in &w {
                for i in j {
                    println!("w={}", i);
                }
            }

            for j in &u {
                for i in j {
                    println!("u={}", i);
                }
            }

            let (delta, mut v) = handle.join().unwrap();

            for i in 0..num {
                u[i][alphas[i]] *= delta;
                for j in 0..u[i].len() {
                    u[i][j] += v[i][j];
                }
            }

            assert_eq!(u, w, "correlation not satisfied");
        }
    }

    #[test]
    fn test_spvole_correlation_h2() {
        for i in vec![1, 2, 5, 10].into_iter() {
            test_spvole_correlation::<2, 4>(i);
        }
    }

    #[test]
    fn test_spvole_correlation_h3() {
        for i in vec![1, 2, 5, 10].into_iter() {
            test_spvole_correlation::<3, 8>(i);
        }
    }

    #[test]
    fn test_spvole_correlation_h4() {
        for i in vec![1, 2, 5, 10].into_iter() {
            test_spvole_correlation::<4, 16>(i);
        }
    }

    #[test]
    fn test_spvole_correlation_h5() {
        for i in vec![1, 2, 5, 10].into_iter() {
            test_spvole_correlation::<5, 32>(i);
        }
    }
}
