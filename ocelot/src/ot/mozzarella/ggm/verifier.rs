use crate::{
    errors::Error,
    ot::{
        mozzarella::ggm::generator::BiasedGen,
        CorrelatedSender,
        RandomSender,
        Sender as OtSender,
    },
};
use rand::{CryptoRng, Rng};
use scuttlebutt::{ring::R64, AbstractChannel, AesHash, AesRng, Block, F128};

use crate::ot::mozzarella::utils;

pub struct Verifier {
    hash: AesHash,
    rng: AesRng,
}

impl Verifier {
    pub fn init() -> Self {
        Self {
            hash: AesHash::new(Default::default()),
            rng: AesRng::new(),
        }
    }

    pub fn gen<const N: usize, const H: usize>(
        &mut self,
        m: &mut [(Block, Block); H],
    ) -> Result<([Block; N], [Block; N], Block), Error> {
        let mut s: [Block; N] = [Block::default(); N];
        let mut final_layer_keys: [Block; N] = [Block::default(); N];
        s[0] = self.rng.gen();

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
                m[i].0 ^= res.0; // keep track of the complete XORs of each layer
                m[i].1 ^= res.1; // keep track of the complete XORs of each layer

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

        Ok((s, final_layer_keys, final_key))
    }

    pub fn send<
        C: AbstractChannel,
        OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender,
        const N: usize,
        const H: usize,
    >(
        &mut self,
        channel: &mut C,
        ot_sender: &mut OT,
        m: &mut [(Block, Block); H],
        final_key: &Block,
    ) -> Result<(), Error> {
        ot_sender.send(channel, &m[..], &mut self.rng)?;
        channel.send(final_key)?;
        Ok(())
    }

    pub fn receive_challenge<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<Block, Error> {
        let seed: Block = channel.receive()?;
        Ok(seed)
    }

    pub fn compute_response<const N: usize, const H: usize>(
        &mut self,
        challenge_seed: Block,
        final_layer_keys: &[Block; N],
    ) -> Result<F128, Error> {
        let mut gen = BiasedGen::new(challenge_seed);
        let mut Gamma = (Block::default(), Block::default());
        for idx in 0..N {
            let xli: F128 = gen.next();
            let cm = xli.cmul(final_layer_keys[idx].into());
            Gamma.0 ^= cm.0;
            Gamma.1 ^= cm.1;
        }
        Ok(F128::reduce(Gamma))
    }

    pub fn send_response<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        response: &F128,
    ) -> Result<(), Error> {
        channel.send(&Block::from(*response))?;
        Ok(())
    }

    pub fn gen_tree<
        C: AbstractChannel,
        OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender,
        const N: usize,
        const H: usize,
    >(
        &mut self,
        channel: &mut C,
        ot_sender: &mut OT,
        m: &mut [(Block, Block); H],
    ) -> Result<[Block; N], Error> {
        let (final_layer_values, final_layer_keys, final_key) = self.gen(m).unwrap();
        self.send::<_, _, N, H>(channel, ot_sender, m, &final_key)?;

        let challenge_seed = self.receive_challenge(channel).unwrap();
        let response = self
            .compute_response::<N, H>(challenge_seed, &final_layer_keys)
            .unwrap();
        self.send_response(channel, &response)?;

        return Ok(final_layer_values);
    }
}
