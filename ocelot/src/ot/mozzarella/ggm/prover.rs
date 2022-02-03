use crate::{
    errors::Error,
    ot::{
        mozzarella::{
            ggm::generator::BiasedGen,
            utils::{log2, prg2, unpack_bits_into},
        },
        CorrelatedReceiver, RandomReceiver, Receiver as OtReceiver,
    },
};
use rand::Rng;
use rayon::prelude::*;
use scuttlebutt::{AbstractChannel, AesHash, AesRng, Block, F128};

pub struct BatchedProver {
    num_instances: usize,
    tree_height: usize,
    output_size: usize,
    _hash: AesHash,
    rng: AesRng,
    alpha_s: Vec<usize>,
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
            alpha_s: vec![0usize; num_instances],
            alpha_bits_s: vec![false; num_instances * tree_height],
            layer_keys_s: vec![Default::default(); num_instances * tree_height],
            final_key_s: vec![Default::default(); num_instances],
            final_layer_blocks_s: vec![Default::default(); num_instances * output_size],
            final_layer_check_values_s: vec![Default::default(); num_instances * output_size],
            challenge_seed_s: vec![Default::default(); num_instances],
            challenge_hash_s: vec![Default::default(); num_instances],
        }
    }

    pub fn new_with_output_size(num_instances: usize, output_size: usize) -> Self {
        let tree_height = log2(output_size);
        assert!(output_size <= 1 << tree_height);

        Self {
            num_instances,
            tree_height,
            output_size,
            _hash: AesHash::new(Default::default()),
            rng: AesRng::new(),
            alpha_s: vec![0usize; num_instances],
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
        self.alpha_s.copy_from_slice(alpha_s);
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
        alpha: usize,
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

        // idea: iteratively treat each key as a root and compute its leafs -- this requires storing a matrix though
        // protocol
        // Start with a single key (either 0 or 1), compute next layer using this
        // Just try to compute as many values as possible. Perhaps fill out the set beforehand with values we know for stuff to not compute?
        // or just check whenever we set the

        // for each layer, note if we are to compute even or odd perhaps?

        // path is easily computable: pack the bits again and use the result as an index
        // compute keyed index using this path: if 1 - alpha[i] = 0 : key = path - 1 else key = path + 1

        // compute the index corresponding to the first key we obtained via OT
        let keyed_index = (alpha >> (tree_height - 1)) ^ 1;
        final_layer_blocks[keyed_index] = layer_keys[0]; // set initial key

        let last_index = output_size - 1;

        // iterate over the tree layer by layer
        for i in 1..(tree_height - 1) {
            // collect XOR of all even/odd keys (depending on the current bit of alpha) to decrypt
            // the key received via OT
            let mut mask = Block::default();
            // expand each node in this layer;
            // we need to iterate from right to left, since we reuse the same buffer
            for j in (0..(last_index >> (tree_height - i)) + 1).rev() {
                // skip the punctured path
                if j == (alpha >> (tree_height - i)) {
                    continue;
                }
                let (s0, s1) = prg2(hash, final_layer_blocks[j]);
                // update the mask
                if !alpha_bits[i] {
                    mask ^= s1;
                } else {
                    mask ^= s0;
                }
                final_layer_blocks[2 * j] = s0;
                final_layer_blocks[2 * j + 1] = s1;
            }
            // decrypt and store neighbor of the node on the punctured path
            let keyed_index = (alpha >> (tree_height - (i + 1))) ^ 1;
            final_layer_blocks[keyed_index] = layer_keys[i] ^ mask;
        }
        // evaluate the last layer
        let mut mask = Block::default();
        // if the last node of the current layer is on the punctured path, we cannot expand
        if (alpha >> 1) != (last_index >> 1) {
            let (s0, s1) = prg2(hash, final_layer_blocks[last_index >> 1]);
            final_layer_blocks[2 * (last_index >> 1)] = s0;
            if alpha_bits[tree_height - 1] {
                mask ^= s0;
            }
            // if the last index is odd, we have to expand the right child
            if last_index & 1 == 1 {
                final_layer_blocks[last_index] = s1;
                if !alpha_bits[tree_height - 1] {
                    mask ^= s1;
                }
            }
        }
        // handle the first nodes normally
        for j in (0..(last_index >> 1)).rev() {
            if j == (alpha >> 1) {
                continue;
            }
            let (s0, s1) = prg2(hash, final_layer_blocks[j]);
            if !alpha_bits[tree_height - 1] {
                mask ^= s1; // keep track of the complete XORs of each layer
            } else {
                mask ^= s0; // keep track of the complete XORs of each layer
            }

            final_layer_blocks[2 * j] = s0;
            final_layer_blocks[2 * j + 1] = s1;
        }
        // decrypt the neighbor of index alpha if it is in bounds
        if alpha < last_index || alpha & 1 == 1 {
            let keyed_index = alpha ^ 1;
            final_layer_blocks[keyed_index] = layer_keys[tree_height - 1] ^ mask;
        }

        // compute the actual outputs and the checking values in the final layer
        let mut last_layer_key = Block::default(); // key for decrypting the check value at index alpha
        for j in 0..output_size {
            if j == alpha {
                continue;
            }

            let (s0, s1) = prg2(hash, final_layer_blocks[j]);
            last_layer_key ^= s1;

            final_layer_blocks[j] = s0;
            final_layer_check_values[j] = F128::from(s1);
        }
        // decrypt the check value at index alpha
        final_layer_check_values[alpha] = F128::from(last_layer_key ^ *final_key);
    }

    pub fn eval(&mut self) {
        let output_size = self.output_size;
        let tree_height = self.tree_height;
        let hash = AesHash::new(Default::default()); // TODO: improve this
        (
            self.alpha_s.par_iter(),
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
                    &alpha,
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
                        alpha,
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
