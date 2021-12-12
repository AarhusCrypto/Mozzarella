use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, Block};
use scuttlebutt::ring::R64;
use crate::Error;
use crate::ot::{CorrelatedSender, FixedKeyInitializer, KosDeltaSender, RandomSender, Sender as OtSender};
use crate::ot::ferret::cache::VecTake;
use crate::ot::mozzarella::cache::verifier::CachedVerifier;
use crate::ot::mozzarella::spvole::verifier::Verifier as spsVerifier;
use crate::ot::mozzarella::utils::flatten;
use crate::ot::mozzarella::lpn::LLCode;
use super::*;

pub struct Verifier{}

impl Verifier {
    pub fn init() -> Self {
        Self{}
    }

    #[allow(non_snake_case)]
    pub fn extend_main<C: AbstractChannel, R: Rng + CryptoRng> (
        channel: &mut C,
        rng: &mut R,
        cache: &mut CachedVerifier, // should be a cache eventually
        sps_verifier: &mut spsVerifier,
        fixed_key: [u8; 16],
    ) -> Result<Vec<R64>, Error> {

        let mut kos18_sender = KosDeltaSender::init_fixed_key(channel, fixed_key, rng)?;

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
            sps_verifier,
            rng,
            channel,
            &mut kos18_sender,
            false,
        )
    }



    #[allow(non_snake_case)]
    pub fn extend<
        OT: OtSender<Msg=Block> + CorrelatedSender + RandomSender,
        C: AbstractChannel,
        R: Rng + CryptoRng,
        const K: usize,
        const N: usize,
        const T: usize,
        const D: usize,
        const LOG_SPLEN: usize,
        const SPLEN: usize,
    >(
        cache: &mut CachedVerifier,
        spvole: &mut spsVerifier,
        rng: &mut R,
        channel: &mut C,
        ot_sender: &mut OT,
        test: bool,
    ) -> Result<Vec<R64>, Error> {
        #[cfg(debug_assertions)]
            {
                debug_assert_eq!(T * SPLEN, N);
            }


        let test_code = &REG_TEST_CODE;
        let code =  &REG_MAIN_CODE;

        let num = T;
        let b: Vec<[R64; SPLEN]> = spvole.extend::<_,_,_, SPLEN, LOG_SPLEN>(channel, rng, num, ot_sender, cache)?;

        let mut b_flat = flatten::<R64, SPLEN>(&b[..]);

        /*for i in &cached_voles[0] {
            println!("VERIFIER_VK:\t {}", i)
        }*/

        // For now we only have a single iteration, so we only need K (hence cached_voles[0]
        let k_cached: Vec<R64> = cache.get(K);
        let mut out: Vec<R64> = Vec::new();
        //if test {
        //    out = test_code.mul_add(&k_cached[..], &mut b_flat);
        //} else {
        out = code.mul_add(&k_cached[..], &mut b_flat);
        //}

        //return Ok(vec![R64(0)])
        return Ok(out);
    }
}