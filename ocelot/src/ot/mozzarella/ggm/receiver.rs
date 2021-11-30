use crate::errors::Error;
use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, Block, AesHash};
use scuttlebutt::ring::R64;

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

    #[allow(non_snake_case)]
    pub fn gen_eval<C: AbstractChannel, RNG: CryptoRng + Rng>(
        &mut self,
        channel: &mut C,
        rng: &mut RNG,
        alphas: &[bool; 4],
        K: &mut Vec<Block>,
    ) -> Result<(Vec<R64>, usize), Error>{
        const N: usize = 16;
        const H: usize = 4;
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
        let mut keyed_index: usize= 0;

        // keep track of the current path index as well as keyed index -- can likely be optimised to avoid the two shifts
        let index = if alphas[0] {1} else {0};
        path_index += index;
        keyed_index = if 1 - index == 0 {path_index - 1} else {path_index + 1};

        out[keyed_index] = K[0]; // set initial key
        //println!("INFO:\tComputing Keyed Index ({}): {}", keyed_index, out[keyed_index]);
        for i in 1..H {
            let mut j = (1 << i) - 1;
            loop {
                if j == path_index {
                    //println!("NOTICE_ME:\tI'M IN HERE!!!!");
                    if j == 0 {
                        break
                    }
                    j -= 1;
                    continue;
                }

                let (s0, s1) = prg2(&self.hash, out[j]);
                if !alphas[i] {
                    m[i] ^= s1; // keep track of the complete XORs of each layer
                } else {
                    m[i] ^= s0; // keep track of the complete XORs of each layer
                }
                //println!("DEBUG:\tValue of m ({}): {}", if alphas[0] { 2 * j } else { 2 * j + 1 }, m[0]);
                out[2 * j] = s0;
                out[2 * j + 1] = s1;

                if j == 0 {
                    break;
                }
                j -= 1;
            }

            let index = if alphas[i] { 1 } else { 0 };
            path_index = 0;
            for tmp in 0..i + 1 {
                let alpha_tmp = if alphas[i - tmp] { 1 } else { 0 };
                path_index += alpha_tmp * (1 << (tmp));
            }
            keyed_index = if 1 - index == 0 { path_index - 1 } else { path_index + 1 };

            //println!("DEBUG:\tXORing {} ^ {}", K[i], m[i - 1]);
            out[keyed_index] = K[i] ^ m[i];
            //println!("INFO:\tComputing Keyed Index ({}): {}", keyed_index, out[keyed_index]);

        }

        //for i in out {
        //    println!("INFO:\tOut: {}", i);
        //}

        return Ok((out.iter().map(|x| R64::from(x.extract_0_u64())).collect(), path_index));
    }
}
