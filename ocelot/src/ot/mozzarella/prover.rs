use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, AesHash};
use scuttlebutt::ring::R64;
use crate::Error;
use crate::ot::mozzarella::spvole::prover::Prover as spProver;
use super::*;

pub struct Prover {
    spvole: spProver,
}

impl Prover {
    pub fn init() -> Self {
        let mut spvole = spProver::init();
        // setup the cache
        Self {
           spvole,
        }
    }

    pub fn vole<C: AbstractChannel, R: Rng + CryptoRng>(
        &mut self,
        channel: &mut C,
        rng: &mut R,
        base_voles: &mut [((R64, R64),(R64, R64))], // should be a cache eventually
        cached_voles: &mut Vec<[(R64, R64); REG_MAIN_K]>, // a vector of K-sized (should be arrays) slices,
    ) -> Result<(Vec<R64>, Vec<R64>), Error> {
        // check if we have any saved in a cache
        let (x, z) = mozzarella::prover::Prover::extend_main(channel, rng, base_voles, cached_voles, &mut self.spvole)?;
        println!("PROVER_OUTPUT:\t x={}, z={}", x[0],z[0]);

        return Ok((x, z))
    }
}