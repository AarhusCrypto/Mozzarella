use crate::errors::Error;
use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, Block, AesHash};

use crate::ot::mozzarella::utils;


pub struct Receiver {
    hash: AesHash,
}

impl Receiver {
    pub fn init() -> Self {
        Self {
            hash: AesHash::new(Default::default()),
        }
    }

    pub fn gen_eval<C: AbstractChannel, RNG: CryptoRng + Rng, const H: usize, const N: usize>(
        &mut self,
        channel: &mut C,
        rng: &mut RNG,
        alphas: &mut [bool; N],
        K: &mut [Block; H],
,
    ) -> Result<Vec<Block>, Error>{


        return Ok(vec!(Block::default()));
    }
}
