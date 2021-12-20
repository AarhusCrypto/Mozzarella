use lazy_static::lazy_static;
use crate::ot::mozzarella::lpn::LLCode;
use scuttlebutt::Block;
use super::*;

pub mod prover;
pub mod verifier;





#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::{Builder, JoinHandle, spawn};
    use rand::{rngs::StdRng, Rng, SeedableRng};
    use rand::rngs::OsRng;
    use scuttlebutt::{Block, unix_channel_pair};
    use scuttlebutt::ring::R64;
    use crate::ot::{FixedKeyInitializer, KosDeltaReceiver, KosDeltaSender, Receiver};
    use crate::ot::mozzarella::cache::cacheinit::GenCache;
    use crate::ot::mozzarella::cache::verifier::CachedVerifier;
    use crate::ot::mozzarella::spvole::prover::Prover as spProver;
    use crate::ot::mozzarella::spvole::verifier::Verifier as spVerifier;
    use crate::ot::mozzarella::mozzarella::prover::Prover;
    use crate::ot::mozzarella::mozzarella::verifier::Verifier;
    use crate::ot::mozzarella::utils::random_array;


    #[test]
    fn test_vole_correlation() {
        //let mut root = StdRng::seed_from_u64(0x5367_FA32_72B1_8478);
        const SPLEN: usize = REG_MAIN_SPLEN;
        const N: usize = REG_MAIN_N;
        const LOG_SPLEN: usize = REG_MAIN_LOG_SPLEN;
        const D: usize = CODE_D;
        const T: usize = REG_MAIN_T;
        const K: usize = REG_MAIN_K;
        const CACHE_SIZE: usize = reg_vole_required(K, T);


        for _ in 0..10 {

            let handler: JoinHandle<()> = Builder::new().stack_size(16*1024*1024).spawn(move || {
                let mut root = StdRng::seed_from_u64(0x5367_FA32_72B1_8443);

                // de-randomize the test
                let mut rng1 = StdRng::seed_from_u64(root.gen());
                let mut rng2 = StdRng::seed_from_u64(root.gen());

                let fixed_key: Block = rng1.gen();
                let delta: R64 = R64(fixed_key.extract_0_u64()); // fyfy, TODO

                println!("DELTA:\t {}", delta);

                let (mut c1, mut c2) = unix_channel_pair();

                // let T=50, so we have enough lol
                let (mut cached_prover, mut cached_verifier) =
                    GenCache::new::<_, 0, CACHE_SIZE>(&mut rng2, delta);
                let handle = Builder::new().stack_size(16*1024*1024).spawn(move || {
                    let mut kos18_send =
                        KosDeltaSender::init_fixed_key(&mut c2, fixed_key.into(), &mut rng2).unwrap();

                    let mut sp_verifier: spVerifier = spVerifier::init(delta);

                    let mut v = Verifier::extend::<_, _, _, K, N, T, D, LOG_SPLEN, SPLEN>(&mut cached_verifier,
                                                                                          &mut sp_verifier,
                                                                                          &mut rng2,
                                                                                          &mut c2,
                                                                                          &mut kos18_send).unwrap();

                    let mut idx = 0;
                    for j in &v {
                        println!("v[{}]={}", idx, j);
                        idx += 1;
                    }

                    println!("delta={}", delta);

                    (delta, v)
                }).unwrap();

                let mut kos18_rec = KosDeltaReceiver::init(&mut c1, &mut rng1).unwrap();


                let mut sp_prover: spProver = spProver::init();

                //( let out = recv.receive_random(&mut c1, &[true], &mut OsRng).unwrap();

                let mut alphas = [0usize; T]; // just sample too many alphas ..
                for e in alphas.iter_mut() {
                    let tmp = rng1.gen::<usize>() % SPLEN;
                    println!("alpha value: {}", tmp);
                    *e = tmp;
                }


                let (mut x, mut z) = Prover::extend::<_, _, _, K, N, T, D, LOG_SPLEN, SPLEN>(&mut cached_prover,
                                                                                             &mut sp_prover,
                                                                                             &mut rng1,
                                                                                             &mut c1,
                                                                                             &mut alphas,
                                                                                             &mut kos18_rec).unwrap();


                let mut idx = 0;
                for j in &z {
                    println!("w[{}]={}", idx, j);
                    idx += 1;
                }

                let mut idx = 0;
                for j in &x {
                    println!("u[{}]={}", idx, j);
                    idx += 1;
                }

                let (delta, mut v) = handle.join().unwrap();

                for i in 0..N {
                    x[i] *= delta;
                    v[i] += x[i];
                }

                for i in 0..N {
                    println!("I:{}", i);
                    assert_eq!(v[i], z[i], "correlation not satisfied");
                }
            }).unwrap();
        }
    }
}