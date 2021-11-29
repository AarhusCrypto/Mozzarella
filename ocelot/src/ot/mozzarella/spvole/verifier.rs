use std::iter::Sum;
use crate::errors::Error;
use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, AesHash, Block, F128};
use crate::ot::mozzarella::ggm::sender as ggmSender;
use crate::ot::{Sender as OtSender, FixedKeyInitializer, RandomSender, CorrelatedSender};

use super::*;
use std::ptr::null;
use scuttlebutt::ring::R64;
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
    ) ->Result<Vec<Vec<R64>>, Error> {
        const N: usize = 8; // tmp
        const H: usize = 3; //tmp
        assert_eq!(1 << H, N);
        //let base_vole = vec![1,2,3]; // tmp -- should come from some cache and be .. actual values

        // create result vector
        let mut vs: Vec<[Block;8]> = Vec::with_capacity(num); // make stuff array as quicker
        unsafe { vs.set_len(num) };
        let mut result: [Block; N] = [Block::default(); N]; // tmp
        //let bs: Vec<usize> = channel.receive_n(num)?;
        println!("INFO:\tReceiver called!");

        // generate the trees before, as we must now use OT to deliver the keys
        // this was not required in ferret, as they could mask the bits instead!
        for rep in 0..num {
            let gamma = R64(1337); // tmp, should be based on something we received earlier
            // used in the computation of "m"
            //let q = &cot[H * rep..H * (rep + 1)];

            let mut m: [(Block, Block); H] = [(Default::default(), Default::default()); H];
            //let mut s: [R64; N] = [Default::default(); N];

            // call the GGM sender and get the m and s
            let mut ggm_sender = ggmSender::Sender::init();

            println!("INFO:\tGenerating GGM tree ...");
            let s: [Block; 8] = ggm_sender.gen_tree(channel, rng, &mut m)?;
            println!("INFO:\tGenerated GGM tree");
            vs[rep] = s;

            ot_sender.send(channel, &m, rng);
            //println!("NOTICE_ME:\tLOL1");
            let tmp: [Block;8] = vs[rep].clone();
            let lol:[R64;8] = tmp.map(|x| R64::from(x.extract_0_u64()));
            for i in lol {
                println!("NOTICE_ME:\t (Verifier) R64={}", i);
            }
            // compute d = gamma - \sum_{i \in [n]} v[i]
            let mut d: R64 = gamma;

            //println!("NOTICE_ME:\tLOL2");

            d -= R64::sum(lol.to_vec().into_iter()); // this sucks
            println!("NOTICE_ME:\td={}", d);

            channel.send(&d);






        }
        let mut k = R64(2);
        return Ok(vec![vec![k]]);

    }
}