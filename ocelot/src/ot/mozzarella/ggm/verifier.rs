use crate::errors::Error;
use rand::{CryptoRng, Rng};
use scuttlebutt::{Block, AesHash, AbstractChannel};
use scuttlebutt::ring::R64;
use crate::ot::{CorrelatedSender, RandomSender, Sender as OtSender};

use crate::ot::mozzarella::utils;


pub struct Verifier {
    hash: AesHash,
}

impl Verifier {
    pub fn init() -> Self{
        Self {
            hash: AesHash::new(Default::default()),
        }
    }

    pub fn gen_tree<
        C: AbstractChannel,
        RNG: CryptoRng + Rng,
        OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender,
        const N: usize, const H: usize>(
        &mut self,
        channel: &mut C,
        ot_sender: &mut OT,
        rng: &mut RNG,
        m: &mut [(Block, Block); H],
    ) -> Result<[Block;N], Error>{

        let mut s: [Block; N] = [Block::default(); N];
        let mut final_layer_keys: [Block; N] = [Block::default(); N];
        s[0] = rng.gen();


        /*
            STEPS:
            1) Compute length-doubling prg for each node until the last layer
            2) Compute "final_prg" for the last layer, resulting in 2*N elements
         */


        // for the final layer we need to treat the elements as field elements, but this is done by
        // simply taking mod 2^k I guess of the additions. Currently this things loops all the way
        // to H, but we should stop earlier if we do not do the final step to make sure it's also secure!
        // this assumes it's secure.. Steps d-f is missing to compute the values that would be used to verify
        for i in 0..H {
            let mut j = (1 << i) - 1;
            loop {
                let res = utils::prg2(&self.hash, s[j]);
                //println!("DEBUG:\tXORing {} ^ {} =", m[i].0, res.0);
                m[i].0 ^= res.0; // keep track of the complete XORs of each layer
                //println!("DEBUG:\tResult: {}", m[i].0);
                //println!("DEBUG:\tXORing {} ^ {} =", m[i].1, res.1);
                m[i].1 ^= res.1; // keep track of the complete XORs of each layer
                //println!("DEBUG:\tResult: {}", m[i].1);


                s[2 * j] = res.0;
                //println!("INFO:\ti:{}\tWriting to {}", i, s[2*j]);
                s[2 * j + 1] = res.1;
                //println!("INFO:\ti:{}\tWriting to {}", i, s[2*j+1]);
                if j == 0 {
                    break;
                }
                j -= 1;
            }
        }

        let mut final_key = Block::default();
        // compute the final layer
        let mut j = (1 << H) - 1;
        loop {
            let res = utils::prg2(&self.hash, s[j]);
            final_key ^= res.1; // keep track of the complete XORs of each layer
            s[j] = res.0;
            final_layer_keys[j] = res.1;
            if j == 0 {
                break;
            }
            j -= 1;
        }

        ot_sender.send(channel, &m[..], rng).unwrap();
        channel.send(&[final_key]).unwrap();


        return Ok(s);
    }
}