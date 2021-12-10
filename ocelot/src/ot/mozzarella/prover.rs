use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, AesHash};
use scuttlebutt::ring::R64;
use crate::Error;
use crate::ot::mozzarella::cache::prover::CachedProver;
use crate::ot::mozzarella::spvole::prover::Prover as spProver;
use super::*;

pub struct Prover {
    spvole: spProver,
    cache: CachedProver,
}

impl Prover {
    pub fn init(cache: CachedProver) -> Self {
        let mut spvole = spProver::init();
        // setup the cache
        Self {
           spvole,
            cache,
        }
    }

    pub fn vole<C: AbstractChannel, R: Rng + CryptoRng>(
        &mut self,
        channel: &mut C,
        rng: &mut R,
    ) -> Result<(R64, R64), Error> {

        if self.cache.capacity() == REG_MAIN_VOLE {
            // replenish using main iteration
            let (x, z) = mozzarella::prover::Prover::extend_main(
                channel,
                rng,
                &mut self.cache,
                &mut self.spvole
            )?;

            dbg!("FILLING UP THE CACHE!");
            self.cache.append(x.into_iter(), z.into_iter());
        }

        /*for i in &x {
            println!("PROVER_OUTPUT_X:\t x={}", i);
        }

        for i in &z {
            println!("PROVER_OUTPUT_Z:\t z={}", i);
        }*/


        let (x,z) = self.cache.pop();
        println!("PROVER_OUTPUT:\t x={}, z={}", x,z);

        return Ok((x, z))
    }
}