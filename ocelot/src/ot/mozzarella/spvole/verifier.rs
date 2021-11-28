use crate::errors::Error;
use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, AesHash, Block, F128};
use crate::ot::mozzarella::ggm::sender as ggmSender;
use crate::ot::{Sender as OtSender, FixedKeyInitializer, RandomSender, CorrelatedSender};

use super::*;
use std::ptr::null;
use scuttlebutt::utils::unpack_bits;

pub struct Verifier {
    pub delta: Block, // tmp
    l: usize, // repetitions of spvole
}

impl Verifier {
    pub fn init(delta: Block) -> Self {
        Self {
            delta,
            l: 0,
        }
    }

    pub fn extend<
        OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender,
        C: AbstractChannel, RNG: CryptoRng + Rng>(
        &mut self,
        channel: &mut C,
        rng: &mut RNG,
        num: usize, // number of repetitions
        ot_sender: &mut OT,
    ) ->Result<Vec<[Block; 16]>, Error> {
        const N: usize = 16; // tmp
        const H: usize = 4; //tmp
        assert_eq!(1 << H, N);
        //let base_vole = vec![1,2,3]; // tmp -- should come from some cache and be .. actual values

        // create result vector
        let mut vs: Vec<[Block; N]> = Vec::with_capacity(num);
        unsafe { vs.set_len(num) };
        let mut result: [Block; N] = [Block::default(); N]; // tmp
        //let bs: Vec<usize> = channel.receive_n(num)?;
        println!("INFO:\tReceiver called!");

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
            println!("INFO:\tGenerating GGM tree ...");
            ggm_sender.gen_tree(channel, rng, &mut m, &mut s); // fix this later -- it currently fills up the m and s
            println!("INFO:\tGenerated GGM tree");
            vs[rep] = s;

            ot_sender.send(channel, &m, rng);

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
            result = s;

        }

        return Ok(vec![result]);

    }
}