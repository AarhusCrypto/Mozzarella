use crate::{
    errors::Error,
    ot::{
        mozzarella::{ggm::generator::BiasedGen, utils::unpack_bits_into_vec},
        CorrelatedReceiver,
        RandomReceiver,
        Receiver as OtReceiver,
    },
};
use rand::Rng;
use scuttlebutt::{AbstractChannel, AesHash, AesRng, Block, F128};

use crate::ot::mozzarella::utils::prg2;

pub struct Prover {
    tree_height: usize,
    output_size: usize,
    hash: AesHash,
    rng: AesRng,
    alpha: usize,
    alpha_bits: Vec<bool>,
    layer_keys: Vec<Block>,
    final_key: Block,
    final_layer_blocks: Vec<Block>,
    final_layer_check_values: Vec<F128>,
    challenge_seed: Block,
    challenge_hash: F128,
}

impl Prover {
    pub fn new(tree_height: usize) -> Self {
        let output_size = 1 << tree_height;
        Self {
            tree_height,
            output_size,
            hash: AesHash::new(Default::default()),
            rng: AesRng::new(),
            alpha: 0,
            alpha_bits: vec![false; tree_height],
            layer_keys: vec![Default::default(); tree_height],
            final_key: Default::default(),
            final_layer_blocks: vec![Default::default(); output_size],
            final_layer_check_values: vec![Default::default(); output_size],
            challenge_seed: Default::default(),
            challenge_hash: Default::default(),
        }
    }

    pub fn get_output_blocks(&self) -> &[Block] {
        self.final_layer_blocks.as_slice()
    }

    #[allow(non_snake_case)]
    pub fn receive<
        C: AbstractChannel,
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
    >(
        &mut self,
        channel: &mut C,
        ot_receiver: &mut OT,
        alpha: usize,
    ) -> Result<(), Error> {
        assert!(alpha < self.output_size);
        self.alpha = alpha;
        unpack_bits_into_vec(alpha, &mut self.alpha_bits); // TODO: fix order
        let ot_input: Vec<bool> = self.alpha_bits.iter().map(|x| !x).collect();
        self.layer_keys = ot_receiver.receive(channel, &ot_input, &mut self.rng)?;
        self.final_key = channel.receive()?;
        Ok(())
    }

    #[allow(non_snake_case)]
    pub fn eval(&mut self) {
        let mut out = vec![Block::default(); self.output_size]; // TODO: reuse self.final_layer_blocks

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
        let index = if self.alpha_bits[0] { 1 } else { 0 };
        path_index += index;
        keyed_index = if 1 - index == 0 {
            path_index - 1
        } else {
            path_index + 1
        };

        out[keyed_index] = self.layer_keys[0]; // set initial key
                                               //println!("INFO:\tComputing Keyed Index ({}): {}", keyed_index, out[keyed_index]);
        for i in 1..self.tree_height {
            let mut m = Block::default();
            let mut j = (1 << i) - 1;
            loop {
                if j == path_index {
                    if j == 0 {
                        break;
                    }
                    j -= 1;
                    continue;
                }

                let (s0, s1) = prg2(&self.hash, out[j]);
                if !self.alpha_bits[i] {
                    m ^= s1; // keep track of the complete XORs of each layer
                } else {
                    m ^= s0; // keep track of the complete XORs of each layer
                }
                //println!("DEBUG:\tValue of m ({}): {}", if alphas[0] { 2 * j } else { 2 * j + 1 }, m[0]);

                out[2 * j] = s0;
                out[2 * j + 1] = s1;

                if j == 0 {
                    break;
                }
                j -= 1;
            }

            let index = if self.alpha_bits[i] { 1 } else { 0 };
            path_index = 0;
            for tmp in 0..i + 1 {
                let alpha_tmp = if self.alpha_bits[i - tmp] { 1 } else { 0 };
                path_index += alpha_tmp * (1 << (tmp));
            }
            keyed_index = if 1 - index == 0 {
                path_index - 1
            } else {
                path_index + 1
            };

            //println!("DEBUG:\tXORing {} ^ {}", K[i], m[i - 1]);
            out[keyed_index] = self.layer_keys[i] ^ m;
            //println!("INFO:\tComputing Keyed Index ({}): {}", keyed_index, out[keyed_index]);
        }

        // compute final layer
        let mut j = (1 << self.tree_height) - 1;
        let mut last_layer_key = Block::default();
        loop {
            if j == path_index {
                if j == 0 {
                    break;
                }
                j -= 1;
                continue;
            }

            let (s0, s1) = prg2(&self.hash, out[j]);
            last_layer_key ^= s1;

            self.final_layer_blocks[j] = s0;
            self.final_layer_check_values[j] = F128::from(s1);

            if j == 0 {
                break;
            }
            j -= 1;
        }

        self.final_layer_check_values[path_index] = F128::from(last_layer_key ^ self.final_key);
    }

    #[allow(non_snake_case)]
    pub fn send_challenge<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(), Error> {
        self.challenge_seed = self.rng.gen();
        // send a seed from which all the changes are derived
        channel.send(&self.challenge_seed)?;
        Ok(())
    }

    #[allow(non_snake_case)]
    pub fn compute_hash(&mut self) {
        let mut gen = BiasedGen::new(self.challenge_seed);
        /*
        TODO: Can we do this for all the ggm trees at once, so we gen n GGM trees
           and then check them all by the end of the protocol?
        */
        // THIS CODE COMPUTES \sum_ i \in [n] xi_i * c_i
        let mut Gamma = (Block::default(), Block::default()); // defer GF(2^128) reduction
        for cv in self.final_layer_check_values.iter() {
            let xli: F128 = gen.next();
            let cm = xli.cmul(*cv);
            Gamma.0 ^= cm.0;
            Gamma.1 ^= cm.1;
        }
        self.challenge_hash = F128::reduce(Gamma);
    }

    #[allow(non_snake_case)]
    pub fn receive_response_and_check<C: AbstractChannel>(&self, channel: &mut C) -> bool {
        let Gamma_prime: Block = channel.receive().unwrap();
        assert_eq!(
            Block::from(self.challenge_hash.clone()),
            Gamma_prime,
            "THE GAMMAS WERE NOT EQUAL!"
        );
        Block::from(self.challenge_hash.clone()) == Gamma_prime
    }

    #[allow(non_snake_case)]
    pub fn gen_eval<
        C: AbstractChannel,
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
    >(
        &mut self,
        channel: &mut C,
        ot_receiver: &mut OT,
        alpha: usize,
    ) -> Result<(), Error> {
        self.receive(channel, ot_receiver, alpha)?;
        self.eval();
        self.send_challenge(channel)?;
        self.compute_hash();

        if self.receive_response_and_check::<C>(channel) {
            return Ok(());
        } else {
            Err(Error::Other("THE GAMMAS WERE NOT EQUAL!".to_string()))
        }
    }
}
