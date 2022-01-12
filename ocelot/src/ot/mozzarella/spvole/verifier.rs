use crate::{
    errors::Error,
    ot::{
        mozzarella::ggm::verifier as ggmVerifier,
        CorrelatedSender,
        FixedKeyInitializer,
        KosDeltaSender,
        RandomSender,
        Sender as OtSender,
    },
};
use rand::{rngs::OsRng, CryptoRng, Rng, SeedableRng};
use scuttlebutt::{
    commitment::{Commitment, ShaCommitment},
    AbstractChannel,
    AesRng,
    Block,
};
use std::convert::TryInto;

use crate::ot::mozzarella::cache::verifier::CachedVerifier;
use itertools::izip;
use rayon::prelude::*;
use scuttlebutt::ring::R64;

#[allow(non_snake_case)]
pub struct SingleVerifier {
    output_size: usize,
    ggm_verifier: ggmVerifier::Verifier,
    Delta: R64,
    a_prime: R64,
    b: R64,
    gamma: R64,
    d: R64,
    chi_seed: Block,
    VV: R64,
    is_init_done: bool,
}

#[allow(non_snake_case)]
impl SingleVerifier {
    pub fn new(log_output_size: usize) -> Self {
        let output_size = 1 << log_output_size;
        Self {
            output_size,
            ggm_verifier: ggmVerifier::Verifier::new(log_output_size),
            Delta: Default::default(),
            a_prime: Default::default(),
            b: Default::default(),
            gamma: Default::default(),
            d: Default::default(),
            chi_seed: Default::default(),
            VV: Default::default(),
            is_init_done: false,
        }
    }

    pub fn init(&mut self, Delta: R64) {
        self.Delta = Delta;
        self.is_init_done = true;
    }

    pub fn stage_1_computation(&mut self, out_v: &mut [R64], base_vole: &[R64; 2]) {
        assert!(self.is_init_done);
        assert_eq!(out_v.len(), self.output_size);
        self.b = base_vole[0];
        self.ggm_verifier.gen();
        let blocks = self.ggm_verifier.get_output_blocks();
        for (v, x) in out_v.iter_mut().zip(blocks) {
            *v = R64::from(x.extract_0_u64());
        }
        self.d = R64::default() - out_v.iter().sum::<R64>();
    }

    pub fn stage_2a_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        self.a_prime = channel.receive()?;
        Ok(())
    }

    pub fn stage_2b_communication<
        C: AbstractChannel,
        OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender,
    >(
        &mut self,
        channel: &mut C,
        ot_sender: &mut OT,
    ) -> Result<(), Error> {
        self.ggm_verifier.send(channel, ot_sender)?;
        Ok(())
    }

    pub fn stage_2c_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        self.ggm_verifier.receive_challenge(channel)?;
        Ok(())
    }

    pub fn stage_2_communication<
        C: AbstractChannel,
        OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender,
    >(
        &mut self,
        channel: &mut C,
        ot_sender: &mut OT,
    ) -> Result<(), Error> {
        self.stage_2a_communication(channel)?;
        self.stage_2b_communication(channel, ot_sender)?;
        self.stage_2c_communication(channel)?;
        Ok(())
    }

    pub fn stage_3_computation(&mut self) {
        self.gamma = self.b - self.Delta * self.a_prime;
        self.d += self.gamma;
        self.ggm_verifier.compute_response();
    }

    pub fn stage_4a_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        self.ggm_verifier.send_response(channel)?;
        channel.send(&self.d)?;
        Ok(())
    }

    pub fn stage_4b_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        self.chi_seed = channel.receive()?;
        Ok(())
    }

    pub fn stage_4_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        self.stage_4a_communication(channel)?;
        self.stage_4b_communication(channel)?;
        Ok(())
    }

    pub fn stage_5_computation(&mut self, out_v: &[R64]) {
        assert_eq!(out_v.len(), self.output_size);
        // expand seed into bit vector chi
        // TODO: optimise to be "roughly" N/2
        let chi: Vec<bool> = {
            let mut indices = vec![false; self.output_size];
            let mut new_rng = AesRng::from_seed(self.chi_seed);

            // N will always be even
            let mut i = 0;
            while i < self.output_size / 2 {
                let tmp: usize = new_rng.gen_range(0, self.output_size);
                if indices[tmp] {
                    continue;
                }
                indices[tmp] = true;
                i += 1;
            }
            indices
        };
        self.VV = chi
            .iter()
            .zip(out_v.iter())
            .filter(|x| *x.0)
            .map(|x| x.1)
            .sum::<R64>();
    }
    pub fn stage_6_communication<C: AbstractChannel, RNG: Rng + CryptoRng>(
        &mut self,
        channel: &mut C,
        base_vole: &[R64; 2],
        rng: &mut RNG,
    ) -> Result<(), Error> {
        let x_star: R64 = channel.receive()?;
        // let y_star = self.base_vole[2 * self.index + 1];
        let y_star = base_vole[1];
        let y: R64 = y_star - self.Delta * x_star;
        self.VV -= y;

        let commitment_randomness: [u8; 32] = rng.gen();
        let committed_VV = {
            let mut com = ShaCommitment::new(commitment_randomness);
            com.input(&self.VV.0.to_le_bytes());
            com.finish()
        };
        channel.send(&committed_VV)?;
        let VP = channel.receive()?;
        channel.send(&self.VV)?;
        channel.send(&commitment_randomness)?;

        if self.VV != VP {
            Err(Error::EqCheckFailed)
        } else {
            Ok(())
        }
    }

    pub fn extend<
        C: AbstractChannel,
        OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender,
    >(
        &mut self,
        channel: &mut C,
        ot_sender: &mut OT,
        out_v: &mut [R64],
        base_vole: &[R64; 2],
    ) -> Result<(), Error> {
        let mut rng = OsRng;
        self.stage_1_computation(out_v, base_vole);
        self.stage_2_communication(channel, ot_sender)?;
        self.stage_3_computation();
        self.stage_4_communication(channel)?;
        self.stage_5_computation(out_v);
        self.stage_6_communication(channel, base_vole, &mut rng)?;

        Ok(())
    }
}

#[allow(non_snake_case)]
pub struct Verifier {
    num_sp_voles: usize,
    single_sp_len: usize,
    total_sp_len: usize,
    single_verifiers: Vec<SingleVerifier>,
    ot_sender: Option<KosDeltaSender>,
    is_init_done: bool,
}

impl Verifier {
    #[allow(non_snake_case)]
    pub fn new(num_sp_voles: usize, log_sp_len: usize) -> Self {
        let single_sp_len = 1 << log_sp_len;
        let total_sp_len = single_sp_len * num_sp_voles;

        // let mut single_verifiers = Vec::<SingleVerifier>::new();
        let single_verifiers: Vec<SingleVerifier> = (0..num_sp_voles)
            .map(|_| SingleVerifier::new(log_sp_len))
            .collect();

        Self {
            num_sp_voles,
            single_sp_len,
            total_sp_len,
            single_verifiers,
            ot_sender: None,
            is_init_done: false,
        }
    }

    pub fn init<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        ot_key: &[u8; 16],
    ) -> Result<(), Error> {
        let mut rng = AesRng::new();
        self.ot_sender = Some(KosDeltaSender::init_fixed_key(channel, *ot_key, &mut rng)?);
        let delta: R64 = R64(Block::from(*ot_key).extract_0_u64());
        self.single_verifiers.iter_mut().for_each(|sv| {
            sv.init(delta);
        });
        self.is_init_done = true;
        Ok(())
    }

    #[allow(non_snake_case)]
    pub fn extend<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        cache: &mut CachedVerifier,
        out_v: &mut [R64],
    ) -> Result<(), Error> {
        assert!(self.is_init_done);
        assert_eq!(out_v.len(), self.total_sp_len);

        let base_vole = cache.get(2 * self.num_sp_voles);
        assert_eq!(base_vole.len(), 2 * self.num_sp_voles);

        let mut rng = OsRng;

        izip!(
            self.single_verifiers.iter_mut(),
            out_v.chunks_exact_mut(self.single_sp_len),
            base_vole.as_slice().chunks_exact(2),
        )
        .par_bridge()
        .for_each(|(sv_i, out_v_i, base_vole_i)| {
            sv_i.stage_1_computation(out_v_i, base_vole_i.try_into().unwrap());
        });
        let ot_sender = self.ot_sender.as_mut().unwrap();
        self.single_verifiers.iter_mut().for_each(|sv_i| {
            sv_i.stage_2_communication(channel, ot_sender).unwrap();
        });
        self.single_verifiers.par_iter_mut().for_each(|sv_i| {
            sv_i.stage_3_computation();
        });
        self.single_verifiers.iter_mut().for_each(|sv_i| {
            sv_i.stage_4_communication(channel).unwrap();
        });
        self.single_verifiers
            .iter_mut()
            .zip(out_v.chunks_exact(self.single_sp_len))
            .par_bridge()
            .for_each(|(sv_i, out_v_i)| {
                sv_i.stage_5_computation(out_v_i);
            });
        self.single_verifiers
            .iter_mut()
            .zip(base_vole.as_slice().chunks_exact(2))
            .for_each(|(sv_i, base_vole_i)| {
                sv_i.stage_6_communication(channel, base_vole_i.try_into().unwrap(), &mut rng)
                    .unwrap();
            });

        Ok(())
    }
}
