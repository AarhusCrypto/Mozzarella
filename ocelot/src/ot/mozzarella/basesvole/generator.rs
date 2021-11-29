use rand::{CryptoRng, Rng, RngCore};
use scuttlebutt::{AbstractChannel, Block};
use scuttlebutt::ring::R64;

pub struct Generator {
    delta: R64, // s = 64
}

impl Generator {


    pub fn init_verifier(&mut self, delta: R64) {
        self.delta = delta;
    }


    // only a single party calls this -- what is security
    pub fn extend<RNG: CryptoRng + Rng>(
        &mut self,
        rng: &mut RNG,
        num: usize,
        seed: usize,
    ) {

    }
}