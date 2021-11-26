use crate::errors::Error;
use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, Block, AesHash};

use crate::ot::mozzarella::utils;


pub struct Sender {
    hash: AesHash,
}

impl Sender {
    pub fn init() -> Self{
        Self {
            hash: AesHash::new(Default::default()),
        }
    }

    pub fn gen_tree<C: AbstractChannel, RNG: CryptoRng + Rng, const H: usize, const N: usize>(
        &mut self,
        channel: &mut C,
        rng: &mut RNG,
        m: &mut [(Block, Block); H],
        s: &mut [Block; N],
    ) {
        //let q = &cot[H * rep..H * (rep + 1)];

        s[0] = rng.gen();
        println!("s[0] = {}", s[0]);

        // for the final layer we need to treat the elements as field elements, but this is done by
        // simply taking mod 2^k I guess of the additions. Currently this things loops all the way
        // to H, but we should stop earlier if we do not do the final step to make sure it's also secure!
        // this assumes it's secure.. Steps d-f is missing to compute the values that would be used to verify
        for i in 0..H {
            let mut j = (1 << i) - 1;
            loop {
                let res = utils::prg2(&self.hash, s[j]);
                m[i].0 ^= res.0; // keep track of the complete XORs of each layer
                m[i].1 ^= res.1; // keep track of the complete XORs of each layer
                s[2 * j] = res.0;
                println!("INFO:\tWriting to {}", 2*j);
                s[2 * j + 1] = res.1;
                println!("INFO:\tWriting to {}", 2*j+1);
                if j == 0 {
                    break;
                }
                j -= 1;
            }
            println!("LEAF LEFT VAL:\t {}", m[i].0);
            println!("LEAF RIGHT VAL:\t {}", m[i].1);
        }

        for i in s {
            println!("INFO:\ts: {}", i);
        }

        //return (m, s);
    }
}