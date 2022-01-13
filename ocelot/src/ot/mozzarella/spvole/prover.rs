use crate::{
    ot::{
        mozzarella::{cache::prover::CachedProver, ggm::prover as ggmProver},
        KosDeltaReceiver,
        Receiver as OtReceiver,
    },
    Error,
};
use rand::{Rng, RngCore, SeedableRng};
use rayon::prelude::*;
use scuttlebutt::{
    commitment::{Commitment, ShaCommitment},
    ring::R64,
    AbstractChannel,
    AesRng,
    Block,
};
use std::time::Instant;

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
        (
            self.ggm_prover.get_output_blocks().par_iter(),
            out_w.par_iter_mut(),
        )
            .into_par_iter()
            .for_each(|(b_i, w_i)| {
                *w_i = R64::from(b_i.extract_0_u64());
            });
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

    #[allow(non_snake_case)]
    fn stage_5_computation_helper(
        output_size: usize,
        out_w: &mut [R64],
        base_vole: (R64, R64),
        alpha: usize,
        delta: R64,
        d: R64,
        chi_seed: Block,
        x_star: &mut R64,
        beta: R64,
        VP: &mut R64,
    ) {
        assert_eq!(out_w.len(), output_size);
        let w_alpha: R64 = delta - d - out_w.iter().sum();
        out_w[alpha] = w_alpha;

        // expand seed to bit vector chi with Hamming weight N/2
        let chi: Vec<bool> = {
            let mut indices = vec![false; output_size];
            let mut new_rng = AesRng::from_seed(chi_seed);

            // TODO: approximate rather than strictly require N/2
            // N will always be even
            let mut i = 0;
            while i < output_size / 2 {
                let tmp: usize = new_rng.gen_range(0, output_size);
                if indices[tmp] {
                    continue;
                }
                indices[tmp] = true;
                i += 1;
            }
            indices
        };

        let chi_alpha: R64 = R64(if chi[alpha] { 1 } else { 0 });
        let x = base_vole.0;
        let z = base_vole.1;

        *x_star = chi_alpha * beta - x;

        // TODO: apparently map is quite slow on large arrays -- is our use-case "large"?
        *VP = chi
            .iter()
            .zip(out_w.iter())
            .filter(|x| *x.0)
            .map(|x| x.1)
            .sum::<R64>()
            - z;
    }

    #[allow(non_snake_case)]
    pub fn stage_5_computation(&mut self, out_w: &mut [R64], base_vole: (&[R64], &[R64])) {
        assert_eq!(out_w.len(), self.num_instances * self.output_size);
        assert_eq!(base_vole.0.len(), self.num_instances * 2);
        assert_eq!(base_vole.1.len(), self.num_instances * 2);

        let output_size = self.output_size;
        (
            out_w.par_chunks_exact_mut(self.output_size),
            base_vole.0[self.num_instances..].par_iter(),
            base_vole.1[self.num_instances..].par_iter(),
            self.alpha_s.par_iter(),
            self.delta_s.par_iter(),
            self.d_s.par_iter(),
            self.chi_seed_s.par_iter(),
            self.x_star_s.par_iter_mut(),
            self.beta_s.par_iter(),
            self.VP_s.par_iter_mut(),
        )
            .into_par_iter()
            .for_each(
                |(
                    out_w,
                    &base_vole_0,
                    &base_vole_1,
                    &alpha,
                    &delta,
                    &d,
                    &chi_seed,
                    x_star,
                    &beta,
                    VP,
                )| {
                    Self::stage_5_computation_helper(
                        output_size,
                        out_w,
                        (base_vole_0, base_vole_1),
                        alpha,
                        delta,
                        d,
                        chi_seed,
                        x_star,
                        beta,
                        VP,
                    );
                },
            );
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
        if (
            VV_s.par_iter(),
            self.VP_s.par_iter(),
            self.committed_VV_s.par_iter(),
            commitment_randomness_s.par_iter(),
        )
            .into_par_iter()
            .all(|(VV, VP, committed_VV, &commitment_randomness)| {
                let recomputed_commitment = {
                    let mut com = ShaCommitment::new(commitment_randomness);
                    com.input(&VV.0.to_le_bytes());
                    com.finish()
                };
                (recomputed_commitment == *committed_VV) && (VV == VP)
            })
        {
            Ok(())
        } else {
            Err(Error::EqCheckFailed)
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
