use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, AesHash};
use scuttlebutt::ring::R64;
use crate::Error;
use crate::ot::mozzarella::spvole::verifier::Verifier as spVerifier;
use super::*;

pub struct Verifier {
    delta: R64,
    spvole: spVerifier,
    ot_key: [u8; 16],
}

impl Verifier {
    pub fn init(delta: R64, fixed_key: [u8; 16],
    ) -> Self {
        // this thing should sample the delta, but for now I need it
        // to generate the base voles we need to bootstrap
        let mut spvole = spVerifier::init(delta);
        Self {
            delta,
            spvole,
            ot_key: fixed_key,
        }
    }

    pub fn vole<C: AbstractChannel, R: Rng + CryptoRng>(
        &mut self,
        channel: &mut C,
        rng: &mut R,
        base_voles: &mut [(R64, R64)], // should be a cache eventually
        cached_voles: &mut Vec<[R64; REG_MAIN_K]>, // should be a cache eventually
    ) -> Result<Vec<R64>, Error>{
        // check if we have any saved in a cache
        let y = mozzarella::verifier::Verifier::extend_main(channel, rng, base_voles, cached_voles, &mut self.spvole, self.ot_key)?;

        for i in &y {
            println!("VERIFER_OUTPUT_Y:\t y={}", i);
        }
        return Ok(y)
    }
}