use std::convert::TryInto;
use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, Block};
use scuttlebutt::ring::R64;
use super::*;

use crate::Error;
// TODO: combine all of these use crate::ot
use crate::ot::{CorrelatedReceiver, KosDeltaReceiver, RandomReceiver, Receiver as OtReceiver};
use crate::ot::mozzarella::cache::prover::CachedProver;

use crate::ot::mozzarella::spvole::prover::Prover as spsProver;
use crate::ot::mozzarella::utils::{flatten, flatten_mut, random_array};
use crate::ot::mozzarella::lpn::LLCode;

pub struct Prover{}

impl Prover {
    pub fn init() -> Self {
        Self{}
    }

    #[allow(non_snake_case)]
    pub fn extend_main<C: AbstractChannel, R: Rng + CryptoRng> (
        channel: &mut C,
        rng: &mut R,
        cache: &mut CachedProver,
        sps_prover: &mut spsProver,
    ) -> Result<(Vec<R64>, Vec<R64>), Error> {
        let mut kos18_receiver = KosDeltaReceiver::init(channel, rng)?;
        let mut alphas: [usize; REG_MAIN_T] = random_array::<_, REG_MAIN_T>(rng, REG_MAIN_SPLEN);

        Self::extend::<
            _,
            _,
            _,
            REG_MAIN_K,
            REG_MAIN_N,
            REG_MAIN_T,
            CODE_D,
            REG_MAIN_LOG_SPLEN,
            REG_MAIN_SPLEN,
        >(
            cache,
            sps_prover,
            rng,
            channel,
            &mut alphas,
            &mut kos18_receiver,
            false,
        )
    }

    #[allow(non_snake_case)]
    pub fn extend<
        OT: OtReceiver<Msg=Block> + CorrelatedReceiver + RandomReceiver,
        C: AbstractChannel,
        R: Rng + CryptoRng,
        const K: usize,
        const N: usize,
        const T: usize,
        const D: usize,
        const LOG_SPLEN: usize,
        const SPLEN: usize,
    >(
        cache: &mut CachedProver,
        spvole: &mut spsProver,
        rng: &mut R,
        channel: &mut C,
        alphas: &mut [usize; T], // error-positions of each spsvole
        ot_receiver: &mut OT,
        test: bool,
    ) -> Result<(Vec<R64>, Vec<R64>), Error> {

        #[cfg(debug_assertions)]
            {
                debug_assert_eq!(T * SPLEN, N);
                for i in alphas.iter().copied() {
                    debug_assert!(i < SPLEN);
                }
            }

        // currently we generate a single VOLE per call to extend
        let test_code = &REG_TEST_CODE;
        let code =  &REG_MAIN_CODE;

        let num = T;
        // have spsvole.extend run multiple executions
        let (mut w, u): (Vec<[R64;SPLEN]>, Vec<[R64; SPLEN]>) = spvole.extend::<_,_,_, SPLEN, LOG_SPLEN>(channel, rng, num, ot_receiver, cache, alphas)?;

        let e_flat = flatten::<R64, SPLEN>(&u[..]);
        let mut c_flat = flatten_mut::<SPLEN>(&mut w[..]);

        /*
        for i in e_flat {
            println!("PROVER_DEBUG:\t e_flat={}", i);
        }

        for i in c_flat {
            println!("PROVER_DEBUG:\t c_flat={}", i);
        }*/




        let mut u_k: [R64; K] = [R64::default(); K];
        let mut w_k: [R64; K] = [R64::default(); K];
        let (u_tmp, w_tmp): (Vec<R64>, Vec<R64>) = cache.get(K);

        for i in 0..K {
            u_k[i] = u_tmp[i];
            //println!("PROVER_UK:\t u_k[{}]={}",idx, i.0 );
            w_k[i] = w_tmp[i];
            //println!("PROVER_WK:\t w_k[{}]={}",idx, i.1);
        }

        // compute x = A*u (and saves into c)
        let mut x: Vec<R64> = Vec::new();
        //if test {
        //    x = test_code.mul(&u_k);
        //} else {
            x = code.mul(&u_k);
        //}

        /*for i in &x {
            println!("BEFORE_ERROR:\t x={}", i);
        }*/



        // if we just remember the different alphas (which we do), we can just quickly compute the correct index instead
        //for (c, i) in x.chunks_exact_mut(SPLEN).zip(alphas.iter().copied()) {
        //    c[i] += e_flat[i];
        //}

        // apparently the above out-documented code broke something for all alphas except the 1st
        let mut idx = 0;
        for i in alphas {
            let alpha = (SPLEN*idx) + *i;
            x[alpha] += e_flat[alpha];
            idx += 1;
        }


        /*for i in &x {
            println!("AFTER_ERROR:\t x={}", i);
        }*/


        // works?


        let z = code.mul_add(&w_k, c_flat);



        //return Ok((vec![R64(0)],vec![R64(0)]));
        return Ok((x, z));
    }
}