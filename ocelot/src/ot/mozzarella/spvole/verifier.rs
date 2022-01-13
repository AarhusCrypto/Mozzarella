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
use std::time::Instant;
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
    VP: R64,
    commitment_randomness: [u8; 32],
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
            VP: Default::default(),
            commitment_randomness: Default::default(),
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

    pub fn stage_6a_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        base_vole: &[R64; 2],
    ) -> Result<(), Error> {
        let x_star: R64 = channel.receive()?;
        // let y_star = self.base_vole[2 * self.index + 1];
        let y_star = base_vole[1];
        let y: R64 = y_star - self.Delta * x_star;
        self.VV -= y;
        Ok(())
    }

    pub fn stage_6b_communication<C: AbstractChannel, RNG: Rng + CryptoRng>(
        &mut self,
        channel: &mut C,
        rng: &mut RNG,
    ) -> Result<(), Error> {
        self.commitment_randomness = rng.gen();
        let committed_VV = {
            let mut com = ShaCommitment::new(self.commitment_randomness);
            com.input(&self.VV.0.to_le_bytes());
            com.finish()
        };
        channel.send(&committed_VV)?;
        Ok(())
    }

    pub fn stage_6c_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        self.VP = channel.receive()?;
        Ok(())
    }

    pub fn stage_6d_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        channel.send(&self.VV)?;
        channel.send(&self.commitment_randomness)?;

        if self.VV != self.VP {
            Err(Error::EqCheckFailed)
        } else {
            Ok(())
        }
    }

    pub fn stage_6_communication<C: AbstractChannel, RNG: Rng + CryptoRng>(
        &mut self,
        channel: &mut C,
        base_vole: &[R64; 2],
        rng: &mut RNG,
    ) -> Result<(), Error> {
        self.stage_6a_communication(channel, base_vole)?;
        self.stage_6b_communication(channel, rng)?;
        self.stage_6c_communication(channel)?;
        self.stage_6d_communication(channel)?;
        Ok(())
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
            sv_i.stage_2a_communication(channel).unwrap();
        });
        self.single_verifiers.iter_mut().for_each(|sv_i| {
            sv_i.stage_2b_communication(channel, ot_sender).unwrap();
        });
        self.single_verifiers.iter_mut().for_each(|sv_i| {
            sv_i.stage_2c_communication(channel).unwrap();
        });

        self.single_verifiers.par_iter_mut().for_each(|sv_i| {
            sv_i.stage_3_computation();
        });

        self.single_verifiers.iter_mut().for_each(|sv_i| {
            sv_i.stage_4a_communication(channel).unwrap();
        });
        self.single_verifiers.iter_mut().for_each(|sv_i| {
            sv_i.stage_4b_communication(channel).unwrap();
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
                sv_i.stage_6a_communication(channel, base_vole_i.try_into().unwrap())
                    .unwrap();
            });
        self.single_verifiers.iter_mut().for_each(|sv_i| {
            sv_i.stage_6b_communication(channel, &mut rng).unwrap();
        });
        self.single_verifiers.iter_mut().for_each(|sv_i| {
            sv_i.stage_6c_communication(channel).unwrap();
        });
        self.single_verifiers.iter_mut().for_each(|sv_i| {
            sv_i.stage_6d_communication(channel).unwrap();
        });

        Ok(())
    }
}

#[allow(non_snake_case)]
pub struct BatchedVerifier {
    num_instances: usize,
    output_size: usize,
    total_output_size: usize,
    ggm_verifier: ggmVerifier::BatchedVerifier,
    ot_sender: Option<KosDeltaSender>,
    Delta: R64,
    a_prime_s: Vec<R64>,
    b_s: Vec<R64>,
    gamma_s: Vec<R64>,
    d_s: Vec<R64>,
    chi_seed_s: Vec<Block>,
    VV_s: Vec<R64>,
    VP_s: Vec<R64>,
    commitment_randomness_s: Vec<[u8; 32]>,
    is_init_done: bool,
}

impl BatchedVerifier {
    pub fn new(num_instances: usize, log_output_size: usize) -> Self {
        let output_size = 1 << log_output_size;
        Self {
            num_instances,
            output_size,
            total_output_size: num_instances * output_size,
            ggm_verifier: ggmVerifier::BatchedVerifier::new(num_instances, log_output_size),
            ot_sender: None,
            Delta: Default::default(),
            a_prime_s: vec![Default::default(); num_instances],
            b_s: vec![Default::default(); num_instances],
            gamma_s: vec![Default::default(); num_instances],
            d_s: vec![Default::default(); num_instances],
            chi_seed_s: vec![Default::default(); num_instances],
            VV_s: vec![Default::default(); num_instances],
            VP_s: vec![Default::default(); num_instances],
            commitment_randomness_s: vec![Default::default(); num_instances],
            is_init_done: false,
        }
    }

    #[allow(non_snake_case)]
    pub fn init<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        ot_key: &[u8; 16],
    ) -> Result<(), Error> {
        let mut rng = AesRng::new();
        self.ot_sender = Some(KosDeltaSender::init_fixed_key(channel, *ot_key, &mut rng)?);
        self.Delta = R64(Block::from(*ot_key).extract_0_u64());
        self.is_init_done = true;
        Ok(())
    }

    pub fn stage_1_computation(&mut self, out_v: &mut [R64], base_vole: &[R64]) {
        assert!(self.is_init_done);
        assert_eq!(out_v.len(), self.num_instances * self.output_size);
        assert_eq!(base_vole.len(), self.num_instances * 2);
        self.b_s.copy_from_slice(&base_vole[..self.num_instances]);
        self.ggm_verifier.gen();
        let blocks = self.ggm_verifier.get_output_blocks();
        for (v, x) in out_v.iter_mut().zip(blocks) {
            *v = R64::from(x.extract_0_u64());
        }
        for inst_i in 0..self.num_instances {
            self.d_s[inst_i] = -out_v[inst_i * self.output_size..(inst_i + 1) * self.output_size]
                .iter()
                .sum::<R64>();
        }
    }

    pub fn stage_2_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        channel.receive_into(self.a_prime_s.as_mut_slice())?;
        self.ggm_verifier
            .send(channel, self.ot_sender.as_mut().unwrap())?;
        self.ggm_verifier.receive_challenge(channel)?;
        Ok(())
    }

    pub fn stage_3_computation(&mut self) {
        for inst_i in 0..self.num_instances {
            self.gamma_s[inst_i] = self.b_s[inst_i] - self.Delta * self.a_prime_s[inst_i];
            self.d_s[inst_i] += self.gamma_s[inst_i];
        }
        self.ggm_verifier.compute_response();
    }

    pub fn stage_4_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        self.ggm_verifier.send_response(channel)?;
        channel.send(self.d_s.as_slice())?;
        channel.receive_into(self.chi_seed_s.as_mut_slice())?;
        Ok(())
    }

    pub fn stage_5_computation(&mut self, out_v: &[R64]) {
        assert_eq!(out_v.len(), self.num_instances * self.output_size);
        for inst_i in 0..self.num_instances {
            // expand seed into bit vector chi
            // TODO: optimise to be "roughly" N/2
            let chi: Vec<bool> = {
                let mut indices = vec![false; self.output_size];
                let mut new_rng = AesRng::from_seed(self.chi_seed_s[inst_i]);

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
            self.VV_s[inst_i] = chi
                .iter()
                .zip(out_v[inst_i * self.output_size..(inst_i + 1) * self.output_size].iter())
                .filter(|x| *x.0)
                .map(|x| x.1)
                .sum::<R64>();
        }
    }

    #[allow(non_snake_case)]
    pub fn stage_6_communication<C: AbstractChannel, RNG: Rng + CryptoRng>(
        &mut self,
        channel: &mut C,
        base_vole: &[R64],
        rng: &mut RNG,
    ) -> Result<(), Error> {
        assert_eq!(base_vole.len(), 2 * self.num_instances);
        let x_star_s: Vec<R64> = channel.receive_n(self.num_instances)?;
        let y_star_s = &base_vole[self.num_instances..];
        let mut committed_VV_s = vec![[0u8; 32]; self.num_instances];
        for inst_i in 0..self.num_instances {
            let y: R64 = y_star_s[inst_i] - self.Delta * x_star_s[inst_i];
            self.VV_s[inst_i] -= y;
            self.commitment_randomness_s[inst_i] = rng.gen();
            committed_VV_s[inst_i] = {
                let mut com = ShaCommitment::new(self.commitment_randomness_s[inst_i]);
                com.input(&self.VV_s[inst_i].0.to_le_bytes());
                com.finish()
            };
        }
        channel.send(committed_VV_s.as_slice())?;
        channel.receive_into(self.VP_s.as_mut_slice())?;
        channel.send(self.VV_s.as_slice())?;
        channel.send(self.commitment_randomness_s.as_slice())?;

        if self.VV_s != self.VP_s {
            Err(Error::EqCheckFailed)
        } else {
            Ok(())
        }
    }

    pub fn extend<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        cache: &mut CachedVerifier,
        out_v: &mut [R64],
    ) -> Result<(), Error> {
        assert!(self.is_init_done);
        assert_eq!(out_v.len(), self.total_output_size);

        let base_vole = cache.get(2 * self.num_instances);
        assert_eq!(base_vole.len(), 2 * self.num_instances);

        let mut rng = OsRng;

        let t_start = Instant::now();
        self.stage_1_computation(out_v, base_vole.as_slice());
        println!("sp-verifier stage 1: {:?}", t_start.elapsed());
        let t_start = Instant::now();
        self.stage_2_communication(channel)?;
        println!("sp-verifier stage 2: {:?}", t_start.elapsed());
        let t_start = Instant::now();
        self.stage_3_computation();
        println!("sp-verifier stage 3: {:?}", t_start.elapsed());
        let t_start = Instant::now();
        self.stage_4_communication(channel)?;
        println!("sp-verifier stage 4: {:?}", t_start.elapsed());
        let t_start = Instant::now();
        self.stage_5_computation(out_v);
        println!("sp-verifier stage 5: {:?}", t_start.elapsed());
        let t_start = Instant::now();
        self.stage_6_communication(channel, base_vole.as_slice(), &mut rng)?;
        println!("sp-verifier stage 6: {:?}", t_start.elapsed());

        Ok(())
    }
}
