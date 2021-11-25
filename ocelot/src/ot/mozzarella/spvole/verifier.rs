use crate::errors::Error;
use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, AesHash, Block, F128};
use crate::ot::mozzarella::ggm::sender as ggmSender;
use crate::ot::{KosDeltaSender, Sender, FixedKeyInitializer};

use super::*;
use std::ptr::null;
use scuttlebutt::utils::unpack_bits;

pub struct Verifier {
    pub delta: u64, // tmp
    l: usize, // repetitions of spvole
}

impl Verifier {
    pub fn init(delta: u64) -> Self {
        Self {
            delta,
            l: 0,
        }
    }

    pub fn extend<C: AbstractChannel, RNG: CryptoRng + Rng, const H: usize, const N: usize>(
        &mut self,
        channel: &mut C,
        rng: &mut RNG,
        num: usize, // number of repetitions
        ot_sender: KosDeltaSender,
    ) ->Result<Vec<[Block; N]>, Error> {
        assert_eq!(1 << H, N);
        //let base_vole = vec![1,2,3]; // tmp -- should come from some cache and be .. actual values

        // create result vector
        let mut vs: Vec<[Block; N]> = Vec::with_capacity(num);
        unsafe { vs.set_len(num) };

        //let bs: Vec<usize> = channel.receive_n(num)?;


        // generate the trees before, as we must now use OT to deliver the keys
        // this was not required in ferret, as they could mask the bits instead!
        for rep in 0..num {
            let gamma = Block::default(); // tmp
            // used in the computation of "m"
            //let q = &cot[H * rep..H * (rep + 1)];

            let mut m: [(Block, Block); H] = [(Default::default(), Default::default()); H];
            let mut s: [Block; N] = [Default::default(); N];

            // call the GGM sender and get the m and s
            let mut ggm_sender = ggmSender::Sender::init();
            ggm_sender.gen_tree(channel, rng, &mut m, &mut s); // fix this later -- it currently fills up the m and s

            vs[rep] = s;

            ot_receiver.receive(channel, &m, rng);

            //let b: [bool; H] = unpack_bits::<H>(b);
            //let l: u128 = (self.l as u128) << 64;

            //for i in 0..H {
                //let tweak: Block = (l | i as u128).into();

                //let h0 = self.hash.tccr_hash(q[i], tweak);
                //let h1 = self.hash.tccr_hash(q[i] ^ self.delta, tweak);

                // M^{i}_{0} := K^{i}_{0} ^ H(q_i ^ b_i D, i || l)
                // M^{i}_{1} := K^{i}_{1} ^ H(q_i ^ !b_i D, i || l)
                // so these are swapped since one of them is multiplied with the inverse bit
                //if b[i] {
                //    m[i].0 ^= h1;
                //    m[i].1 ^= h0;
                //} else {
                //    m[i].0 ^= h0;
                //    m[i].1 ^= h1;
                //}
            //}

            // compute d = gamma - \sum_{i \in [n]} v[i]
            let mut d = gamma;
            //for i in 0..N {
            //    d -= v[i];
            //}
            // take mod of d


            // send (m, c) to R
            //channel.send(&m)?;
            //channel.send(&c)?;
        }



        return Err(Error::Other("consistency check failed".to_owned()));

    }
}