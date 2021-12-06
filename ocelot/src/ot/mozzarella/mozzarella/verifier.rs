use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, Block};
use scuttlebutt::ring::R64;
use crate::Error;
use crate::ot::{CorrelatedSender, FixedKeyInitializer, KosDeltaSender, RandomSender, Sender as OtSender};
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
        base_voles: &mut [(R64,R64)], // should be a cache eventually
        cached_voles: &mut Vec<[R64; REG_MAIN_K]>, // should be a cache eventually
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
            base_voles,
            cached_voles,
            sps_verifier,
            rng,
            channel,
            &mut kos18_sender,
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
        base_voles: &mut [(R64, R64)],
        cached_voles: &mut Vec<[R64; REG_MAIN_K]>,
        spvole: &mut spsVerifier,
        rng: &mut R,
        channel: &mut C,
        ot_sender: &mut OT,
    ) -> Result<Vec<R64>, Error> {
        #[cfg(debug_assertions)]
            {
                debug_assert_eq!(T * SPLEN, N);
            }

        let code=  &REG_MAIN_CODE;
        let num = 1;
        let v: Vec<[R64; SPLEN]> = spvole.extend::<_,_,_,SPLEN, LOG_SPLEN>(channel, rng, num, ot_sender, base_voles)?; // should return SPLEN

        let mut v_flat = flatten::<R64, N>(&v); // maybe works?

        // For now we only have a single iteration, so we only need K (hence cached_voles[0]
        code.mul_add(&cached_voles[0], &mut v_flat);

        return Ok(Vec::from(v_flat));
    }
}