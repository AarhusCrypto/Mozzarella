use crate::{
    errors::Error,
    ot::{
        mozzarella::ggm::generator::BiasedGen,
        CorrelatedSender,
        RandomSender,
        Sender as OtSender,
    },
};
use rand::Rng;
use scuttlebutt::{AbstractChannel, AesHash, AesRng, Block, F128};

use crate::ot::mozzarella::utils;

#[allow(non_snake_case)]
pub struct Verifier {
    tree_height: usize,
    output_size: usize,
    hash: AesHash,
    rng: AesRng,
    layer_key_pairs: Vec<(Block, Block)>,
    final_layer_keys: Vec<Block>,
    final_layer_blocks: Vec<Block>,
    final_key: Block,
    challenge_seed: Block,
    challenge_response: F128,
}

#[allow(non_snake_case)]
impl Verifier {
    pub fn new(tree_height: usize) -> Self {
        let output_size = 1 << tree_height;

        Self {
            tree_height,
            output_size,
            hash: AesHash::new(Default::default()),
            rng: AesRng::new(),
            layer_key_pairs: vec![Default::default(); tree_height],
            final_layer_keys: vec![Default::default(); output_size],
            final_layer_blocks: vec![Default::default(); output_size],
            final_key: Default::default(),
            challenge_seed: Default::default(),
            challenge_response: Default::default(),
        }
    }

    pub fn get_output_blocks(&self) -> &[Block] {
        self.final_layer_blocks.as_slice()
    }

    pub fn gen(&mut self) {
        self.final_layer_blocks[0] = self.rng.gen();

        /*
           STEPS:
           1) Compute length-doubling prg for each node until the last layer
           2) Compute "final_prg" for the last layer, resulting in 2*N elements
        */

        // for the final layer we need to treat the elements as field elements, but this is done by
        // simply taking mod 2^k I guess of the additions. Currently this things loops all the way
        // to H, but we should stop earlier if we do not do the final step to make sure it's also secure!
        // this assumes it's secure.. Steps d-f is missing to compute the values that would be used to verify
        for i in 0..self.tree_height {
            let mut j = (1 << i) - 1;
            loop {
                let res = utils::prg2(&self.hash, self.final_layer_blocks[j]);
                self.layer_key_pairs[i].0 ^= res.0; // keep track of the complete XORs of each layer
                self.layer_key_pairs[i].1 ^= res.1; // keep track of the complete XORs of each layer

                self.final_layer_blocks[2 * j] = res.0;
                //println!("INFO:\ti:{}\tWriting to {}", i, s[2*j]);
                self.final_layer_blocks[2 * j + 1] = res.1;
                //println!("INFO:\ti:{}\tWriting to {}", i, s[2*j+1]);
                if j == 0 {
                    break;
                }
                j -= 1;
            }
        }

        let mut final_key = Block::default();
        // compute the final layer
        let mut j = (1 << self.tree_height) - 1;
        loop {
            let res = utils::prg2(&self.hash, self.final_layer_blocks[j]);
            final_key ^= res.1; // keep track of the complete XORs of each layer
            self.final_layer_blocks[j] = res.0;
            self.final_layer_keys[j] = res.1;
            if j == 0 {
                break;
            }
            j -= 1;
        }
        self.final_key = final_key;
    }

    pub fn send_layer_keys<
        C: AbstractChannel,
        OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender,
    >(
        &mut self,
        channel: &mut C,
        ot_sender: &mut OT,
    ) -> Result<(), Error> {
        debug_assert_eq!(self.layer_key_pairs.len(), self.tree_height);
        ot_sender.send(channel, self.layer_key_pairs.as_slice(), &mut self.rng)?;
        Ok(())
    }

    pub fn send_final_key<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(), Error> {
        channel.send(&self.final_key)?;
        Ok(())
    }

    pub fn send<C: AbstractChannel, OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender>(
        &mut self,
        channel: &mut C,
        ot_sender: &mut OT,
    ) -> Result<(), Error> {
        self.send_layer_keys(channel, ot_sender)?;
        self.send_final_key(channel)?;
        Ok(())
    }

    pub fn receive_challenge<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(), Error> {
        self.challenge_seed = channel.receive()?;
        Ok(())
    }

    pub fn compute_response(&mut self) {
        let mut gen = BiasedGen::new(self.challenge_seed);
        let mut Gamma = (Block::default(), Block::default());
        for idx in 0..self.output_size {
            let xli: F128 = gen.next();
            let cm = xli.cmul(self.final_layer_keys[idx].into());
            Gamma.0 ^= cm.0;
            Gamma.1 ^= cm.1;
        }
        self.challenge_response = F128::reduce(Gamma);
    }

    pub fn send_response<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(), Error> {
        channel.send(&Block::from(self.challenge_response))?;
        Ok(())
    }

    pub fn gen_tree<
        C: AbstractChannel,
        OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender,
    >(
        &mut self,
        channel: &mut C,
        ot_sender: &mut OT,
    ) -> Result<(), Error> {
        self.gen();
        self.send(channel, ot_sender)?;
        self.receive_challenge(channel)?;
        self.compute_response();
        self.send_response(channel)?;
        Ok(())
    }
}

pub struct BatchedVerifier {
    num_instances: usize,
    tree_height: usize,
    output_size: usize,
    hash: AesHash,
    rng: AesRng,
    layer_key_pairs_s: Vec<(Block, Block)>,
    final_layer_keys_s: Vec<Block>,
    final_layer_blocks_s: Vec<Block>,
    final_key_s: Vec<Block>,
    challenge_seed_s: Vec<Block>,
    challenge_response_s: Vec<F128>,
}

impl BatchedVerifier {
    pub fn new(num_instances: usize, tree_height: usize) -> Self {
        let output_size = 1 << tree_height;

        Self {
            num_instances,
            tree_height,
            output_size,
            hash: AesHash::new(Default::default()),
            rng: AesRng::new(),
            layer_key_pairs_s: vec![Default::default(); num_instances * tree_height],
            final_layer_keys_s: vec![Default::default(); num_instances * output_size],
            final_layer_blocks_s: vec![Default::default(); num_instances * output_size],
            final_key_s: vec![Default::default(); num_instances],
            challenge_seed_s: vec![Default::default(); num_instances],
            challenge_response_s: vec![Default::default(); num_instances],
        }
    }

    pub fn get_output_blocks(&self) -> &[Block] {
        self.final_layer_blocks_s.as_slice()
    }

    pub fn gen(&mut self) {
        for tree_i in 0..self.num_instances {
            let tree_output_size_offset = tree_i * self.output_size;
            let tree_height_offset = tree_i * self.tree_height;
            self.final_layer_blocks_s[tree_output_size_offset + 0] = self.rng.gen();

            /*
               STEPS:
               1) Compute length-doubling prg for each node until the last layer
               2) Compute "final_prg" for the last layer, resulting in 2*N elements
            */

            // for the final layer we need to treat the elements as field elements, but this is done by
            // simply taking mod 2^k I guess of the additions. Currently this things loops all the way
            // to H, but we should stop earlier if we do not do the final step to make sure it's also secure!
            // this assumes it's secure.. Steps d-f is missing to compute the values that would be used to verify
            for i in 0..self.tree_height {
                let mut j = (1 << i) - 1;
                loop {
                    let res = utils::prg2(
                        &self.hash,
                        self.final_layer_blocks_s[tree_output_size_offset + j],
                    );
                    self.layer_key_pairs_s[tree_height_offset + i].0 ^= res.0; // keep track of the complete XORs of each layer
                    self.layer_key_pairs_s[tree_height_offset + i].1 ^= res.1; // keep track of the complete XORs of each layer

                    self.final_layer_blocks_s[tree_output_size_offset + 2 * j] = res.0;
                    //println!("INFO:\ti:{}\tWriting to {}", i, s[2*j]);
                    self.final_layer_blocks_s[tree_output_size_offset + 2 * j + 1] = res.1;
                    //println!("INFO:\ti:{}\tWriting to {}", i, s[2*j+1]);
                    if j == 0 {
                        break;
                    }
                    j -= 1;
                }
            }

            let mut final_key = Block::default();
            // compute the final layer
            let mut j = (1 << self.tree_height) - 1;
            loop {
                let res = utils::prg2(
                    &self.hash,
                    self.final_layer_blocks_s[tree_output_size_offset + j],
                );
                final_key ^= res.1; // keep track of the complete XORs of each layer
                self.final_layer_blocks_s[tree_output_size_offset + j] = res.0;
                self.final_layer_keys_s[tree_output_size_offset + j] = res.1;
                if j == 0 {
                    break;
                }
                j -= 1;
            }
            self.final_key_s[tree_i] = final_key;
        }
    }

    pub fn send_layer_keys<
        C: AbstractChannel,
        OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender,
    >(
        &mut self,
        channel: &mut C,
        ot_sender: &mut OT,
    ) -> Result<(), Error> {
        debug_assert_eq!(
            self.layer_key_pairs_s.len(),
            self.num_instances * self.tree_height
        );
        ot_sender.send(channel, self.layer_key_pairs_s.as_slice(), &mut self.rng)?;
        Ok(())
    }

    pub fn send_final_key<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(), Error> {
        channel.send(self.final_key_s.as_slice())?;
        Ok(())
    }

    pub fn send<C: AbstractChannel, OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender>(
        &mut self,
        channel: &mut C,
        ot_sender: &mut OT,
    ) -> Result<(), Error> {
        self.send_layer_keys(channel, ot_sender)?;
        self.send_final_key(channel)?;
        Ok(())
    }

    pub fn receive_challenge<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(), Error> {
        channel.receive_into(self.challenge_seed_s.as_mut_slice())?;
        Ok(())
    }

    pub fn compute_response(&mut self) {
        for tree_i in 0..self.num_instances {
            let tree_output_size_offset = tree_i * self.output_size;
            let mut gen = BiasedGen::new(self.challenge_seed_s[tree_i]);
            let mut capital_gamma = (Block::default(), Block::default());
            for idx in 0..self.output_size {
                let xli: F128 = gen.next();
                let cm = xli.cmul(self.final_layer_keys_s[tree_output_size_offset + idx].into());
                capital_gamma.0 ^= cm.0;
                capital_gamma.1 ^= cm.1;
            }
            self.challenge_response_s[tree_i] = F128::reduce(capital_gamma);
        }
    }

    pub fn send_response<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(), Error> {
        channel.send(self.challenge_response_s.as_slice())?;
        Ok(())
    }

    pub fn gen_tree<
        C: AbstractChannel,
        OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender,
    >(
        &mut self,
        channel: &mut C,
        ot_sender: &mut OT,
    ) -> Result<(), Error> {
        self.gen();
        self.send(channel, ot_sender)?;
        self.receive_challenge(channel)?;
        self.compute_response();
        self.send_response(channel)?;
        Ok(())
    }
}
