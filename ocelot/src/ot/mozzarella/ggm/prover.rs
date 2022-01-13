use crate::{
    errors::Error,
    ot::{
        mozzarella::{ggm::generator::BiasedGen, utils::unpack_bits_into},
        CorrelatedReceiver,
        RandomReceiver,
        Receiver as OtReceiver,
    },
};
use rand::Rng;
use rayon::prelude::*;
use scuttlebutt::{AbstractChannel, AesHash, AesRng, Block, F128};

use crate::ot::mozzarella::utils::prg2;

pub struct Prover {
    tree_height: usize,
    output_size: usize,
    hash: AesHash,
    rng: AesRng,
    alpha_bits: Vec<bool>,
    layer_keys: Vec<Block>,
    final_key: Block,
    final_layer_blocks: Vec<Block>,
    final_layer_check_values: Vec<F128>,
    challenge_seed: Block,
    challenge_hash: F128,
}

#[allow(non_snake_case)]
impl Prover {
    pub fn new(tree_height: usize) -> Self {
        let output_size = 1 << tree_height;
        Self {
            tree_height,
            output_size,
            hash: AesHash::new(Default::default()),
            rng: AesRng::new(),
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

    pub fn receive_layer_keys<
        C: AbstractChannel,
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
    >(
        &mut self,
        channel: &mut C,
        ot_receiver: &mut OT,
        alpha: usize,
    ) -> Result<(), Error> {
        assert!(alpha < self.output_size);
        unpack_bits_into(alpha, self.alpha_bits.as_mut_slice()); // TODO: fix order
        let ot_input: Vec<bool> = self.alpha_bits.iter().map(|x| !x).collect();
        self.layer_keys = ot_receiver.receive(channel, &ot_input, &mut self.rng)?;
        Ok(())
    }
    pub fn receive_final_key<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(), Error> {
        self.final_key = channel.receive()?;
        Ok(())
    }

    pub fn receive<
        C: AbstractChannel,
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
    >(
        &mut self,
        channel: &mut C,
        ot_receiver: &mut OT,
        alpha: usize,
    ) -> Result<(), Error> {
        self.receive_layer_keys(channel, ot_receiver, alpha)?;
        self.receive_final_key(channel)?;
        Ok(())
    }

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

pub struct BatchedProver {
    num_instances: usize,
    tree_height: usize,
    output_size: usize,
    _hash: AesHash,
    rng: AesRng,
    alpha_bits_s: Vec<bool>,
    layer_keys_s: Vec<Block>,
    final_key_s: Vec<Block>,
    final_layer_blocks_s: Vec<Block>,
    final_layer_check_values_s: Vec<F128>,
    challenge_seed_s: Vec<Block>,
    challenge_hash_s: Vec<F128>,
}

impl BatchedProver {
    pub fn new(num_instances: usize, tree_height: usize) -> Self {
        let output_size = 1 << tree_height;
        Self {
            num_instances,
            tree_height,
            output_size,
            _hash: AesHash::new(Default::default()),
            rng: AesRng::new(),
            alpha_bits_s: vec![false; num_instances * tree_height],
            layer_keys_s: vec![Default::default(); num_instances * tree_height],
            final_key_s: vec![Default::default(); num_instances],
            final_layer_blocks_s: vec![Default::default(); num_instances * output_size],
            final_layer_check_values_s: vec![Default::default(); num_instances * output_size],
            challenge_seed_s: vec![Default::default(); num_instances],
            challenge_hash_s: vec![Default::default(); num_instances],
        }
    }

    pub fn get_output_blocks(&self) -> &[Block] {
        self.final_layer_blocks_s.as_slice()
    }

    pub fn receive_layer_keys<
        C: AbstractChannel,
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
    >(
        &mut self,
        channel: &mut C,
        ot_receiver: &mut OT,
        alpha_s: &[usize],
    ) -> Result<(), Error> {
        assert_eq!(alpha_s.len(), self.num_instances);
        for (tree_i, &alpha) in alpha_s.iter().enumerate() {
            assert!(alpha < self.output_size);
            unpack_bits_into(
                alpha,
                &mut self.alpha_bits_s[tree_i * self.tree_height..(tree_i + 1) * self.tree_height],
            ); // TODO: fix order?
        }
        let ot_input: Vec<bool> = self.alpha_bits_s.iter().map(|x| !x).collect();
        self.layer_keys_s = ot_receiver.receive(channel, &ot_input, &mut self.rng)?;
        Ok(())
    }
    pub fn receive_final_key<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(), Error> {
        channel.receive_into(self.final_key_s.as_mut_slice())?;
        Ok(())
    }

    pub fn receive<
        C: AbstractChannel,
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
    >(
        &mut self,
        channel: &mut C,
        ot_receiver: &mut OT,
        alpha_s: &[usize],
    ) -> Result<(), Error> {
        self.receive_layer_keys(channel, ot_receiver, alpha_s)?;
        self.receive_final_key(channel)?;
        Ok(())
    }

    pub fn eval_helper(
        output_size: usize,
        tree_height: usize,
        hash: &AesHash,
        alpha_bits: &[bool],
        final_layer_blocks: &mut [Block],
        final_layer_check_values: &mut [F128],
        layer_keys: &[Block],
        final_key: &Block,
    ) {
        assert_eq!(alpha_bits.len(), tree_height);
        assert_eq!(final_layer_blocks.len(), output_size);
        assert_eq!(final_layer_check_values.len(), output_size);
        assert_eq!(layer_keys.len(), tree_height);

        let mut out = vec![Block::default(); output_size]; // TODO: reuse self.final_layer_blocks

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
        let index = if alpha_bits[0] { 1 } else { 0 };
        path_index += index;
        keyed_index = if 1 - index == 0 {
            path_index - 1
        } else {
            path_index + 1
        };

        out[keyed_index] = layer_keys[0]; // set initial key
                                          //println!("INFO:\tComputing Keyed Index ({}): {}", keyed_index, out[keyed_index]);
        for i in 1..tree_height {
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

                let (s0, s1) = prg2(hash, out[j]);
                if !alpha_bits[i] {
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

            let index = if alpha_bits[i] { 1 } else { 0 };
            path_index = 0;
            for tmp in 0..i + 1 {
                let alpha_tmp = if alpha_bits[i - tmp] { 1 } else { 0 };
                path_index += alpha_tmp * (1 << (tmp));
            }
            keyed_index = if 1 - index == 0 {
                path_index - 1
            } else {
                path_index + 1
            };

            //println!("DEBUG:\tXORing {} ^ {}", K[i], m[i - 1]);
            out[keyed_index] = layer_keys[i] ^ m;
            //println!("INFO:\tComputing Keyed Index ({}): {}", keyed_index, out[keyed_index]);
        }

        // compute final layer
        let mut j = (1 << tree_height) - 1;
        let mut last_layer_key = Block::default();
        loop {
            if j == path_index {
                if j == 0 {
                    break;
                }
                j -= 1;
                continue;
            }

            let (s0, s1) = prg2(hash, out[j]);
            last_layer_key ^= s1;

            final_layer_blocks[j] = s0;
            final_layer_check_values[j] = F128::from(s1);

            if j == 0 {
                break;
            }
            j -= 1;
        }

        final_layer_check_values[path_index] = F128::from(last_layer_key ^ *final_key);
    }

    pub fn eval(&mut self) {
        let output_size = self.output_size;
        let tree_height = self.tree_height;
        let hash = AesHash::new(Default::default()); // TODO: improve this
        (
            self.alpha_bits_s.par_chunks_exact_mut(self.tree_height),
            self.final_layer_blocks_s
                .par_chunks_exact_mut(self.output_size),
            self.final_layer_check_values_s
                .par_chunks_exact_mut(self.output_size),
            self.layer_keys_s.par_chunks_exact(self.tree_height),
            self.final_key_s.par_iter(),
        )
            .into_par_iter()
            .for_each(
                |(
                    alpha_bits,
                    final_layer_blocks,
                    final_layer_check_values,
                    layer_keys,
                    final_key,
                )| {
                    Self::eval_helper(
                        output_size,
                        tree_height,
                        &hash,
                        alpha_bits,
                        final_layer_blocks,
                        final_layer_check_values,
                        layer_keys,
                        final_key,
                    );
                },
            );
    }

    pub fn send_challenge<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(), Error> {
        for cs_i in self.challenge_seed_s.iter_mut() {
            *cs_i = self.rng.gen();
        }
        // send a seed from which all the changes are derived
        channel.send(self.challenge_seed_s.as_slice())?;
        Ok(())
    }

    fn compute_hash_helper(challenge_seed: &Block, final_layer_check_values: &[F128]) -> F128 {
        let mut gen = BiasedGen::new(*challenge_seed);
        /*
        TODO: Can we do this for all the ggm trees at once, so we gen n GGM trees
           and then check them all by the end of the protocol?
        */
        // THIS CODE COMPUTES \sum_ i \in [n] xi_i * c_i
        let mut capital_gamma = (Block::default(), Block::default()); // defer GF(2^128) reduction
        for &cv in final_layer_check_values {
            let xli: F128 = gen.next();
            let cm = xli.cmul(cv);
            capital_gamma.0 ^= cm.0;
            capital_gamma.1 ^= cm.1;
        }
        F128::reduce(capital_gamma)
    }

    pub fn compute_hash(&mut self) {
        (
            self.challenge_seed_s.par_iter(),
            self.final_layer_check_values_s
                .par_chunks_exact(self.output_size),
            self.challenge_hash_s.par_iter_mut(),
        )
            .into_par_iter()
            .for_each(|(challenge_seed, final_layer_keys, challenge_hash)| {
                *challenge_hash = Self::compute_hash_helper(challenge_seed, final_layer_keys);
            });
    }

    pub fn receive_response_and_check<C: AbstractChannel>(&self, channel: &mut C) -> bool {
        let mut capital_gamma_prime_s = vec![F128::default(); self.num_instances];
        channel
            .receive_into(capital_gamma_prime_s.as_mut_slice())
            .unwrap();
        assert_eq!(
            self.challenge_hash_s, capital_gamma_prime_s,
            "THE GAMMAS WERE NOT EQUAL!"
        );
        self.challenge_hash_s == capital_gamma_prime_s
    }

    pub fn gen_eval<
        C: AbstractChannel,
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
    >(
        &mut self,
        channel: &mut C,
        ot_receiver: &mut OT,
        alpha_s: &[usize],
    ) -> Result<(), Error> {
        self.receive(channel, ot_receiver, alpha_s)?;
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
