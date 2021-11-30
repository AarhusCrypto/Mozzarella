use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, Block};
use scuttlebutt::ring::R64;
use crate::Error;
use crate::ot::{CorrelatedSender, RandomSender, Sender as OtSender};
//use crate::ot::mozzarella::lpn::LLCode;
use crate::ot::mozzarella::spvole::verifier::Verifier as spsVerifier;
use crate::ot::mozzarella::utils::flatten;

pub struct Receiver{}

#[allow(non_snake_case)]
pub fn extend<
    OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender,
    C: AbstractChannel,
    R: Rng + CryptoRng,
    const K: usize,
    const N: usize,
    const T: usize,
    const D: usize,
    const LOG_SPLEN: usize,
    const SPLEN: usize,
>(
    //code: &LLCode<K, N, D>,
    base_voles: &mut Vec<R64>,
    spvole: &mut spsVerifier,
    rng: &mut R,
    channel: &mut C,
    ot_sender: &mut OT,
)  -> Result<Vec<R64>, Error>{
    #[cfg(debug_assertions)]
        {
            debug_assert_eq!(T * SPLEN, N);
        }

    let num = 1;
    let v: Vec<[R64; 16]> = spvole.extend(channel, rng, num, ot_sender, base_voles)?; // should return SPLEN

    let v_flat = flatten::<R64, 16>(&v); // maybe works?


    return Ok(vec!(R64(42)));


}