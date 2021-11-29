use crate::errors::Error;
use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, Block, AesHash};
use scuttlebutt::ring::R64;

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

    pub fn gen_tree<C: AbstractChannel, RNG: CryptoRng + Rng>(
        &mut self,
        channel: &mut C,
        rng: &mut RNG,
        m: &mut [(Block, Block); 4],
    ) -> Result<[Block;16], Error>{
        const H: usize = 4;
        const N: usize = 16;
        //let q = &cot[H * rep..H * (rep + 1)];

        //let mut m: [(Block, Block); H] = [(Block::default(), Block::default()); H];
        let mut s: [Block; N] = [Block::default(); N];
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
                println!("DEBUG:\tXORing {} ^ {} =", m[i].0, res.0);
                m[i].0 ^= res.0; // keep track of the complete XORs of each layer
                println!("DEBUG:\tResult: {}", m[i].0);
                println!("DEBUG:\tXORing {} ^ {} =", m[i].1, res.1);
                m[i].1 ^= res.1; // keep track of the complete XORs of each layer
                println!("DEBUG:\tResult: {}", m[i].1);


                s[2 * j] = res.0;
                println!("INFO:\ti:{}\tWriting to {}", i, s[2*j]);
                s[2 * j + 1] = res.1;
                println!("INFO:\ti:{}\tWriting to {}", i, s[2*j+1]);
                if j == 0 {
                    break;
                }
                j -= 1;
            }

            for i in s {
                println!("NOTICE_ME:\ts={}", i);
            }
        }
        return Ok(s);
        //return Ok(s.iter().map(|x| R64::from(x.extract_0_u64())).collect());
    }
}