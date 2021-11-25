use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, Block};
use crate::Error;

pub struct Receiver{}

impl Receiver {

    pub fn extend<C: AbstractChannel, RNG: CryptoRng + Rng, const N: usize, const H: usize>(

    ) -> Result<Vec<Block>, Error> {


        return Ok(vec![Block::default()]);
    }
}