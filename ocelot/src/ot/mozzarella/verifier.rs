use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel};
use scuttlebutt::ring::R64;
use crate::Error;
use crate::ot::mozzarella::cache::verifier::CachedVerifier;
use crate::ot::mozzarella::spvole::verifier::Verifier as spVerifier;
use super::*;

pub struct Verifier {
    spvole: spVerifier,
    ot_key: [u8; 16],
    cache: CachedVerifier,
}

impl Verifier {
    pub fn init(delta: R64, fixed_key: [u8; 16], cache: CachedVerifier) -> Self {
        // this thing should sample the delta, but for now I need it
        // to generate the base voles we need to bootstrap
        let spvole = spVerifier::init(delta);
        Self {
            spvole,
            ot_key: fixed_key,
            cache,
        }
    }

    pub fn vole<C: AbstractChannel, R: Rng + CryptoRng>(
        &mut self,
        channel: &mut C,
        rng: &mut R,
    ) -> Result<R64, Error>{
        // check if we have any saved in a cache
        if self.cache.capacity() == REG_MAIN_VOLE {
            // replenish using main iteration
            let y = mozzarella::verifier::Verifier::extend_main(
                channel,
                rng,
                &mut self.cache,
                &mut self.spvole,
                self.ot_key)?;

            self.cache.append(y.into_iter());
        }

        let out = self.cache.pop();
        return Ok(out)
    }
}