use crate::{
    ot::{
        mozzarella::{cache::prover::CachedProver, ggm::prover as ggmProver},
        CorrelatedReceiver,
        KosDeltaReceiver,
        RandomReceiver,
        Receiver as OtReceiver,
    },
    Error,
};
use std::time::Instant;
use itertools::izip;
use rand::{Rng, RngCore, SeedableRng};
use rayon::prelude::*;
use scuttlebutt::{
    commitment::{Commitment, ShaCommitment},
    ring::R64,
    AbstractChannel,
    AesRng,
    Block,
};
use std::convert::TryInto;

#[allow(non_snake_case)]
pub struct SingleProver {
    output_size: usize,
    ggm_prover: ggmProver::Prover,
    rng: AesRng,
    alpha: usize,
    beta: R64,
    delta: R64,
    a_prime: R64,
    d: R64,
    chi_seed: Block,
    x_star: R64,
    VP: R64,
    committed_VV: [u8; 32],
}

#[allow(non_snake_case)]
impl SingleProver {
    pub fn new(log_output_size: usize) -> Self {
        let output_size = 1 << log_output_size;
        Self {
            output_size,
            ggm_prover: ggmProver::Prover::new(log_output_size),
            rng: AesRng::new(),
            alpha: 0,
            beta: R64::default(),
            delta: R64::default(),
            a_prime: R64::default(),
            d: R64::default(),
            chi_seed: Block::default(),
            x_star: R64::default(),
            VP: R64::default(),
            committed_VV: Default::default(),
        }
    }

    pub fn get_alpha(&self) -> usize {
        self.alpha
    }

    pub fn stage_1_computation(&mut self, out_u: &mut [R64], base_vole: (&[R64; 2], &[R64; 2])) {
        assert_eq!(out_u.len(), self.output_size);
        self.alpha = self.rng.gen_range(0, self.output_size);
        self.chi_seed = self.rng.gen();
        let a = base_vole.0[0];
        let c = base_vole.1[0];
        self.delta = c;
        while self.beta.0 == 0 {
            self.beta = R64(self.rng.next_u64());
        }
        out_u[self.alpha] = self.beta;
        self.a_prime = self.beta - a;
    }

    pub fn stage_2a_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        channel.send(&self.a_prime)?;
        Ok(())
    }

    pub fn stage_2b_communication<
        C: AbstractChannel,
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
    >(
        &mut self,
        channel: &mut C,
        ot_receiver: &mut OT,
    ) -> Result<(), Error> {
        self.ggm_prover.receive(channel, ot_receiver, self.alpha)?;
        Ok(())
    }

    pub fn stage_2c_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        self.ggm_prover.send_challenge(channel)?;
        Ok(())
    }

    pub fn stage_2_communication<
        C: AbstractChannel,
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
    >(
        &mut self,
        channel: &mut C,
        ot_receiver: &mut OT,
    ) -> Result<(), Error> {
        self.stage_2a_communication(channel)?;
        self.stage_2b_communication(channel, ot_receiver)?;
        self.stage_2c_communication(channel)?;
        Ok(())
    }

    pub fn stage_3_computation(&mut self, out_w: &mut [R64]) {
        assert_eq!(out_w.len(), self.output_size);
        self.ggm_prover.eval();
        // TODO: write directly into buffers
        for (i, b_i) in self.ggm_prover.get_output_blocks().iter().enumerate() {
            out_w[i] = R64::from(b_i.extract_0_u64());
        }
        self.ggm_prover.compute_hash();
    }

    pub fn stage_4a_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        if !self.ggm_prover.receive_response_and_check(channel) {
            return Err(Error::Other("THE GAMMAS WERE NOT EQUAL!".to_string()));
        }
        self.d = channel.receive()?;
        Ok(())
    }

    pub fn stage_4b_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        channel.send(&self.chi_seed)?;
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

    pub fn stage_5_computation(&mut self, out_w: &mut [R64], base_vole: (&[R64; 2], &[R64; 2])) {
        assert_eq!(out_w.len(), self.output_size);
        let w_alpha: R64 = self.delta - self.d - out_w.iter().sum();
        out_w[self.alpha] = w_alpha;

        // expand seed to bit vector chi with Hamming weight N/2
        let chi: Vec<bool> = {
            let mut indices = vec![false; self.output_size];
            let mut new_rng = AesRng::from_seed(self.chi_seed);

            // TODO: approximate rather than strictly require N/2
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

        let chi_alpha: R64 = R64(if chi[self.alpha] { 1 } else { 0 });
        let x = base_vole.0[1];
        let z = base_vole.1[1];

        self.x_star = chi_alpha * self.beta - x;

        // TODO: apparently map is quite slow on large arrays -- is our use-case "large"?
        self.VP = chi
            .iter()
            .zip(out_w.iter())
            .filter(|x| *x.0)
            .map(|x| x.1)
            .sum::<R64>()
            - z;
    }

    pub fn stage_6a_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        channel.send(&self.x_star)?;
        Ok(())
    }

    pub fn stage_6b_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        self.committed_VV = channel.receive()?;
        Ok(())
    }

    pub fn stage_6c_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        channel.send(&self.VP)?;
        Ok(())
    }

    pub fn stage_6d_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        let VV: R64 = channel.receive()?;
        let commitment_randomness: [u8; 32] = channel.receive()?;
        let recomputed_commitment = {
            let mut com = ShaCommitment::new(commitment_randomness);
            com.input(&VV.0.to_le_bytes());
            com.finish()
        };

        if recomputed_commitment != self.committed_VV {
            Err(Error::CommitmentInvalidOpening)
        } else if VV != self.VP {
            Err(Error::EqCheckFailed)
        } else {
            Ok(())
        }
    }

    pub fn stage_6_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        self.stage_6a_communication(channel)?;
        self.stage_6b_communication(channel)?;
        self.stage_6c_communication(channel)?;
        self.stage_6d_communication(channel)?;
        Ok(())
    }

    pub fn extend<
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
        C: AbstractChannel,
    >(
        &mut self,
        channel: &mut C,
        ot_receiver: &mut OT,
        out_u: &mut [R64],
        out_w: &mut [R64],
        base_vole: (&[R64; 2], &[R64; 2]),
    ) -> Result<(), Error> {
        self.stage_1_computation(out_u, base_vole);
        self.stage_2_communication(channel, ot_receiver)?;
        self.stage_3_computation(out_w);
        self.stage_4_communication(channel)?;
        self.stage_5_computation(out_w, base_vole);
        self.stage_6_communication(channel)?;
        Ok(())
    }
}

pub struct Prover {
    num_sp_voles: usize,
    single_sp_len: usize,
    total_sp_len: usize,
    single_provers: Vec<SingleProver>,
    ot_receiver: Option<KosDeltaReceiver>,
    is_init_done: bool,
}

impl Prover {
    pub fn new(num_sp_voles: usize, log_sp_len: usize) -> Self {
        let single_sp_len = 1 << log_sp_len;
        let total_sp_len = single_sp_len * num_sp_voles;

        // let mut single_verifiers = Vec::<SingleVerifier>::new();
        let single_provers: Vec<SingleProver> = (0..num_sp_voles)
            .map(|_| SingleProver::new(log_sp_len))
            .collect();

        Self {
            num_sp_voles,
            single_sp_len,
            total_sp_len,
            single_provers,
            ot_receiver: None,
            is_init_done: false,
        }
    }

    pub fn init<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(), Error> {
        let mut rng = AesRng::new();
        self.ot_receiver = Some(KosDeltaReceiver::init(channel, &mut rng)?);
        self.is_init_done = true;
        Ok(())
    }

    #[allow(non_snake_case)]
    pub fn extend<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        cache: &mut CachedProver,
        alphas: &mut [usize],
        out_u: &mut [R64],
        out_w: &mut [R64],
    ) -> Result<(), Error> {
        assert!(self.is_init_done);
        assert_eq!(alphas.len(), self.num_sp_voles);
        assert_eq!(out_u.len(), self.total_sp_len);
        assert_eq!(out_w.len(), self.total_sp_len);

        let base_vole = cache.get(2 * self.num_sp_voles);
        assert_eq!(base_vole.0.len(), 2 * self.num_sp_voles);
        assert_eq!(base_vole.1.len(), 2 * self.num_sp_voles);

        izip!(
            self.single_provers.iter_mut(),
            out_u.chunks_exact_mut(self.single_sp_len),
            base_vole.0.as_slice().chunks_exact(2),
            base_vole.1.as_slice().chunks_exact(2),
        )
        .par_bridge()
        .for_each(|(sp_i, out_u_i, base_vole_i_0, base_vole_i_1)| {
            sp_i.stage_1_computation(
                out_u_i,
                (
                    base_vole_i_0.try_into().unwrap(),
                    base_vole_i_1.try_into().unwrap(),
                ),
            );
        });

        let ot_receiver = self.ot_receiver.as_mut().unwrap();
        self.single_provers.iter_mut().for_each(|sp_i| {
            sp_i.stage_2a_communication(channel).unwrap();
        });
        self.single_provers.iter_mut().for_each(|sp_i| {
            sp_i.stage_2b_communication(channel, ot_receiver).unwrap();
        });
        self.single_provers.iter_mut().for_each(|sp_i| {
            sp_i.stage_2c_communication(channel).unwrap();
        });

        self.single_provers
            .iter_mut()
            .zip(out_w.chunks_exact_mut(self.single_sp_len))
            .par_bridge()
            .for_each(|(sp_i, out_w_i)| {
                sp_i.stage_3_computation(out_w_i);
            });

        self.single_provers.iter_mut().for_each(|sp_i| {
            sp_i.stage_4a_communication(channel).unwrap();
        });
        self.single_provers.iter_mut().for_each(|sp_i| {
            sp_i.stage_4b_communication(channel).unwrap();
        });

        izip!(
            self.single_provers.iter_mut(),
            out_w.chunks_exact_mut(self.single_sp_len),
            base_vole.0.as_slice().chunks_exact(2),
            base_vole.1.as_slice().chunks_exact(2),
        )
        .par_bridge()
        .for_each(|(sp_i, out_w_i, base_vole_i_0, base_vole_i_1)| {
            sp_i.stage_5_computation(
                out_w_i,
                (
                    base_vole_i_0.try_into().unwrap(),
                    base_vole_i_1.try_into().unwrap(),
                ),
            );
        });

        self.single_provers.iter_mut().for_each(|sp_i| {
            sp_i.stage_6a_communication(channel).unwrap();
        });
        self.single_provers.iter_mut().for_each(|sp_i| {
            sp_i.stage_6b_communication(channel).unwrap();
        });
        self.single_provers.iter_mut().for_each(|sp_i| {
            sp_i.stage_6c_communication(channel).unwrap();
        });
        self.single_provers.iter_mut().for_each(|sp_i| {
            sp_i.stage_6d_communication(channel).unwrap();
        });

        for i in 0..self.num_sp_voles {
            alphas[i] = self.single_provers[i].get_alpha();
        }

        Ok(())
    }
}

#[allow(non_snake_case)]
pub struct BatchedProver {
    num_instances: usize,
    output_size: usize,
    total_output_size: usize,
    ggm_prover: ggmProver::BatchedProver,
    ot_receiver: Option<KosDeltaReceiver>,
    rng: AesRng,
    alpha_s: Vec<usize>,
    beta_s: Vec<R64>,
    delta_s: Vec<R64>,
    a_prime_s: Vec<R64>,
    d_s: Vec<R64>,
    chi_seed_s: Vec<Block>,
    x_star_s: Vec<R64>,
    VP_s: Vec<R64>,
    committed_VV_s: Vec<[u8; 32]>,
    is_init_done: bool,
}

impl BatchedProver {
    pub fn new(num_instances: usize, log_output_size: usize) -> Self {
        let output_size = 1 << log_output_size;
        Self {
            num_instances,
            output_size,
            total_output_size: num_instances * output_size,
            ggm_prover: ggmProver::BatchedProver::new(num_instances, log_output_size),
            ot_receiver: None,
            rng: AesRng::new(),
            alpha_s: vec![Default::default(); num_instances],
            beta_s: vec![Default::default(); num_instances],
            delta_s: vec![Default::default(); num_instances],
            a_prime_s: vec![Default::default(); num_instances],
            d_s: vec![Default::default(); num_instances],
            chi_seed_s: vec![Default::default(); num_instances],
            x_star_s: vec![Default::default(); num_instances],
            VP_s: vec![Default::default(); num_instances],
            committed_VV_s: vec![Default::default(); num_instances],
            is_init_done: false,
        }
    }

    pub fn get_alphas(&self) -> &[usize] {
        self.alpha_s.as_slice()
    }

    pub fn init<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(), Error> {
        let mut rng = AesRng::new();
        self.ot_receiver = Some(KosDeltaReceiver::init(channel, &mut rng)?);
        self.is_init_done = true;
        Ok(())
    }

    pub fn stage_1_computation(&mut self, out_u: &mut [R64], base_vole: (&[R64], &[R64])) {
        assert_eq!(out_u.len(), self.num_instances * self.output_size);
        // assert!(out_u.iter().all(|&x| x == R64::default()));
        assert_eq!(base_vole.0.len(), self.num_instances * 2);
        assert_eq!(base_vole.1.len(), self.num_instances * 2);
        for inst_i in 0..self.num_instances {
            self.alpha_s[inst_i] = self.rng.gen_range(0, self.output_size);
            self.chi_seed_s[inst_i] = self.rng.gen();
            let a = base_vole.0[inst_i];
            let c = base_vole.1[inst_i];
            self.delta_s[inst_i] = c;
            while self.beta_s[inst_i].0 == 0 {
                self.beta_s[inst_i] = R64(self.rng.next_u64());
            }
            out_u[inst_i * self.output_size + self.alpha_s[inst_i]] = self.beta_s[inst_i];
            self.a_prime_s[inst_i] = self.beta_s[inst_i] - a;
        }
    }

    pub fn stage_2_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        channel.send(self.a_prime_s.as_slice())?;
        self.ggm_prover.receive(
            channel,
            self.ot_receiver.as_mut().unwrap(),
            self.alpha_s.as_slice(),
        )?;
        self.ggm_prover.send_challenge(channel)?;
        Ok(())
    }

    pub fn stage_3_computation(&mut self, out_w: &mut [R64]) {
        assert_eq!(out_w.len(), self.num_instances * self.output_size);
        self.ggm_prover.eval();
        // TODO: write directly into buffers
        for (i, b_i) in self.ggm_prover.get_output_blocks().iter().enumerate() {
            out_w[i] = R64::from(b_i.extract_0_u64());
        }
        self.ggm_prover.compute_hash();
    }

    pub fn stage_4_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        if !self.ggm_prover.receive_response_and_check(channel) {
            return Err(Error::Other("THE GAMMAS WERE NOT EQUAL!".to_string()));
        }
        channel.receive_into(self.d_s.as_mut_slice())?;
        channel.send(self.chi_seed_s.as_slice())?;
        Ok(())
    }

    pub fn stage_5_computation(&mut self, out_w: &mut [R64], base_vole: (&[R64], &[R64])) {
        assert_eq!(out_w.len(), self.num_instances * self.output_size);
        assert_eq!(base_vole.0.len(), self.num_instances * 2);
        assert_eq!(base_vole.1.len(), self.num_instances * 2);
        for inst_i in 0..self.num_instances {
            let w_alpha: R64 = self.delta_s[inst_i]
                - self.d_s[inst_i]
                - out_w[inst_i * self.output_size..(inst_i + 1) * self.output_size]
                    .iter()
                    .sum();
            out_w[inst_i * self.output_size + self.alpha_s[inst_i]] = w_alpha;

            // expand seed to bit vector chi with Hamming weight N/2
            let chi: Vec<bool> = {
                let mut indices = vec![false; self.output_size];
                let mut new_rng = AesRng::from_seed(self.chi_seed_s[inst_i]);

                // TODO: approximate rather than strictly require N/2
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

            let chi_alpha: R64 = R64(if chi[self.alpha_s[inst_i]] { 1 } else { 0 });
            let x = base_vole.0[self.num_instances + inst_i];
            let z = base_vole.1[self.num_instances + inst_i];

            self.x_star_s[inst_i] = chi_alpha * self.beta_s[inst_i] - x;

            // TODO: apparently map is quite slow on large arrays -- is our use-case "large"?
            self.VP_s[inst_i] = chi
                .iter()
                .zip(out_w[inst_i * self.output_size..(inst_i + 1) * self.output_size].iter())
                .filter(|x| *x.0)
                .map(|x| x.1)
                .sum::<R64>()
                - z;
        }
    }

    #[allow(non_snake_case)]
    pub fn stage_6_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        channel.send(self.x_star_s.as_slice())?;
        channel.receive_into(self.committed_VV_s.as_mut_slice())?;
        channel.send(self.VP_s.as_slice())?;
        let VV_s: Vec<R64> = channel.receive_n(self.num_instances)?;
        let commitment_randomness_s: Vec<[u8; 32]> = channel.receive_n(self.num_instances)?;
        let mut recomputed_commitment_s = vec![[0u8; 32]; self.num_instances];
        for inst_i in 0..self.num_instances {
            recomputed_commitment_s[inst_i] = {
                let mut com = ShaCommitment::new(commitment_randomness_s[inst_i]);
                com.input(&VV_s[inst_i].0.to_le_bytes());
                com.finish()
            };
        }

        if recomputed_commitment_s != self.committed_VV_s {
            Err(Error::CommitmentInvalidOpening)
        } else if VV_s != self.VP_s {
            Err(Error::EqCheckFailed)
        } else {
            Ok(())
        }
    }

    pub fn extend<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        cache: &mut CachedProver,
        alphas: &mut [usize],
        out_u: &mut [R64],
        out_w: &mut [R64],
    ) -> Result<(), Error> {
        assert!(self.is_init_done);
        assert_eq!(alphas.len(), self.num_instances);
        assert_eq!(out_u.len(), self.total_output_size);
        assert_eq!(out_w.len(), self.total_output_size);

        let base_vole = cache.get(2 * self.num_instances);
        assert_eq!(base_vole.0.len(), 2 * self.num_instances);
        assert_eq!(base_vole.1.len(), 2 * self.num_instances);

        let t_start = Instant::now();
        self.stage_1_computation(out_u, (&base_vole.0[..], &base_vole.1[..]));
        println!("sp-prover stage 1: {:?}", t_start.elapsed());
        let t_start = Instant::now();
        self.stage_2_communication(channel)?;
        println!("sp-prover stage 2: {:?}", t_start.elapsed());
        let t_start = Instant::now();
        self.stage_3_computation(out_w);
        println!("sp-prover stage 3: {:?}", t_start.elapsed());
        let t_start = Instant::now();
        self.stage_4_communication(channel)?;
        println!("sp-prover stage 4: {:?}", t_start.elapsed());
        let t_start = Instant::now();
        self.stage_5_computation(out_w, (&base_vole.0[..], &base_vole.1[..]));
        println!("sp-prover stage 5: {:?}", t_start.elapsed());
        let t_start = Instant::now();
        self.stage_6_communication(channel)?;
        println!("sp-prover stage 6: {:?}", t_start.elapsed());
        alphas.copy_from_slice(self.alpha_s.as_slice());
        Ok(())
    }
}
