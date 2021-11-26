use crate::errors::Error;
use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, Block, AesHash};

use crate::ot::mozzarella::utils;
use crate::ot::mozzarella::utils::prg2;


pub struct Receiver {
    hash: AesHash,
}

impl Receiver {
    pub fn init() -> Self {
        Self {
            hash: AesHash::new(Default::default()),
        }
    }

    pub fn gen_eval<C: AbstractChannel, RNG: CryptoRng + Rng>(
        &mut self,
        channel: &mut C,
        rng: &mut RNG,
        alphas: &mut [bool; 9],
        K: &mut Vec<Block>,
    ) -> Result<Vec<Block>, Error>{
        const N: usize = 8;
        const H: usize = 3;
        let mut out: [Block; N] = [Block::default(); N]; // consider making this N-1 to not waste space
        let mut m: [Block ; H] = [Block::default(); H];


        // idea: iteratively treat each key as a root and compute its leafs -- this requires storing a matrix though
        // protocol
        // Start with a single key (either 0 or 1), compute next layer using this
        // Just try to compute as many values as possible. Perhaps fill out the set beforehand with values we know for stuff to not compute?
        // or just check whenever we set the

        // for each layer, note if we are to compute even or odd perhaps?

        // path is easily computable: pack the bits again and use the result as an index
        // compute keyed index using this path: if 1 - alpha[i] = 0 : key = path - 1 else key = path + 1

        let mut path_index: usize = 0;
        let mut keyed_index: usize = 0;
        let mut x: usize = 0;
        for i in 1..H {

            // keep track of the current path index as well as keyed index -- can likely be optimised to avoid the two shifts
            let index = if alphas[i-1] {1} else {0};
            path_index += index * (1 << (i-1));
            keyed_index = if 1 - index == 0 {path_index - 1} else {path_index + 1};

            let mut j = (1 << i) - 1;

            // compute keyed value
            if i-1 != 0 {
                out[keyed_index] = K[i-1] ^ m[i-1];
            } else {
                out[keyed_index] = K[i-1]; // set initial key
            }

            loop {

                if j == path_index {
                    continue;
                }

                let (s0, s1) = prg2(&self.hash, out[j]);
                m[i].index ^= res.index; // keep track of the complete XORs of each layer
                out[2 * j] = s0;
                out[2 * j + 1] = s1;
                if j == 0 {
                    break;
                }
                j -= 1;
            }
        }

        for i in out {
            println!("INFO:\tOut: {}", i);
        }


        return Ok(vec!(Block::default()));
    }
}
