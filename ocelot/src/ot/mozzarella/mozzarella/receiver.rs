use crate::ot::mozzarella::spvole::prover;
use crate::errors::Error;

use scuttlebutt::{AbstractChannel, Block};

use rand::{CryptoRng, Rng};

pub struct Receiver{}

/*pub fn extend<
    C: AbstractChannel,
    R: Rng + CryptoRng,
    const K: usize,
    const N: usize,
    const T: usize,
    const D: usize,
    const LOG_SPLEN: usize,
    const SPLEN: usize,
    >(
    code: &LLCode<K, N, D>,
    base: &mut CachedReceiver,   // base COTs
    spvole: &mut
    rng: &mut Rng,
    channel: &mut C,
    ) -> Result<(Vec<Block>, Vev<Block>), Error> {
}*/