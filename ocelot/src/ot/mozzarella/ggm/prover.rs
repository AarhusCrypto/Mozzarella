use rand::{CryptoRng, Rng};
use sha2::digest::generic_array::typenum::Abs;
use crate::errors::Error;
use scuttlebutt::{Block, AesHash, AbstractChannel, F128};
use scuttlebutt::ring::R64;
use crate::ot::{CorrelatedReceiver, RandomReceiver, Receiver as OtReceiver};
use crate::ot::mozzarella::ggm::generator::BiasedGen;

use crate::ot::mozzarella::utils::prg2;


pub struct Prover {
    hash: AesHash,
}

impl Prover {
    pub fn init() -> Self {
        Self {
            hash: AesHash::new(Default::default()),
        }
    }

    #[allow(non_snake_case)]
    pub fn gen_eval<
        C: AbstractChannel,
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
        RNG: CryptoRng + Rng,
        const N: usize,
        const H: usize> (
        &mut self,
        channel: &mut C,
        ot_receiver: &mut OT,
        rng: &mut RNG,
        alphas: &[bool; H],
    ) -> Result<([R64; N], usize), Error>{


        let ot_input: [bool; H] = alphas.map(|x| !x);
        let mut K: Vec<Block> = ot_receiver.receive(channel, &ot_input, rng)?;
        let final_key = channel.receive().unwrap();



        let mut out: [Block; N]= [Block::default(); N];
        let mut final_layer_values: [R64; N] = [R64::default(); N];
        let mut final_layer_keys: [Block; N] = [Block::default(); N];
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
        let mut keyed_index;

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

        // compute final layer
        let mut j = (1 << H) - 1;
        let mut last_layer_key = Block::default();
        loop {
            if j == path_index {
                if j == 0 {
                    break
                }
                j -= 1;
                continue;
            }

            let (s0, s1) = prg2(&self.hash, out[j]);
            last_layer_key ^= s1;

            final_layer_values[j] = R64(s0.extract_0_u64());
            final_layer_keys[j] = s1;

            if j == 0 {
                break;
            }
            j -= 1;
        }

        final_layer_keys[path_index] = last_layer_key ^ final_key;


        // send a seed from which all the changes are derived
        let seed: Block = rng.gen();
        let mut gen = BiasedGen::new(seed);

        /*
         TODO: Can we do this for all the ggm trees at once, so we gen n GGM trees
            and then check them all by the end of the protocol?
         */
        // THIS CODE COMPUTES \sum_ i \in [n] xi_i * c_i
        let mut Gamma = (Block::default(), Block::default()); // defer GF(2^128) reduction
        for idx in 0..N {
            let xli: F128 = gen.next();
            let cm = xli.cmul(final_layer_keys[idx].into());
            Gamma.0 ^= cm.0;
            Gamma.1 ^= cm.1;
        }

        let Gamma: F128 = F128::reduce(Gamma);

        channel.send(&seed)?;

        let Gamma_prime: Block = channel.receive().unwrap();

        assert_eq!(Gamma.ret_self(), Gamma_prime, "THE GAMMAS WERE NOT EQUAL!");

        return Ok((final_layer_values, path_index));
    }
}
