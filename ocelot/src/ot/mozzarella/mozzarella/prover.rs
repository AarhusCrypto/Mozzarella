use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, Block};
use scuttlebutt::ring::R64;

use crate::Error;
// TODO: combine all of these use crate::ot
use crate::ot::{CorrelatedReceiver, RandomReceiver, Receiver as OtReceiver};
//use crate::ot::mozzarella::lpn::LLCode;
use crate::ot::mozzarella::spvole::prover::Prover as spsProver;
use crate::ot::mozzarella::utils::flatten;

pub struct Receiver{}

#[allow(non_snake_case)]
pub fn extend<
    OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
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
    base_voles: &mut Vec<(R64, R64)>,
    spvole: &mut spsProver,
    rng: &mut R,
    channel: &mut C,
    alphas: &[usize; T], // error-positions of each spsvole
    ot_receiver: &mut OT,
)  -> Result<Vec<R64>, Error>{
    #[cfg(debug_assertions)]
        {
            debug_assert_eq!(T * SPLEN, N);
            for i in alphas.iter().copied() {
                debug_assert!(i < SPLEN);
            }
        }
    let rep = 1;
    let num = 1;
    // have spsvole.extend run multiple executions
    let (w, u): (Vec<[R64; 16]>, Vec<[R64; 16]>) = spvole.extend(channel, rng, num, ot_receiver, base_voles, alphas)?;

    let u_flat = flatten::<R64, 16>(&u); // maybe works?
    let w_flat = flatten::<R64, 16>(&w); // maybe works?


    return Ok(vec!(R64(42)));


}