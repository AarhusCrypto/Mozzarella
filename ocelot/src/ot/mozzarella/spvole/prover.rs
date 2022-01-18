use crate::{
    ot::{
        mozzarella::{cache::prover::CachedProver, ggm::prover as ggmProver},
        KosDeltaReceiver,
        Receiver as OtReceiver,
    },
    Error,
};
use rand::{
    distributions::{Distribution, Standard},
    Rng,
    SeedableRng,
};
use rayon::prelude::*;
use scuttlebutt::{
    channel::{Receivable, Sendable},
    commitment::{Commitment, ShaCommitment},
    ring::NewRing,
    AbstractChannel,
    AesRng,
    Block,
};
use serde::Serialize;
use std::time::{Duration, Instant};

#[allow(non_snake_case)]
pub struct BatchedProver<RingT>
where
    RingT: NewRing + Receivable,
    Standard: Distribution<RingT>,
    for<'a> &'a RingT: Sendable,
{
    num_instances: usize,
    output_size: usize,
    total_output_size: usize,
    ggm_prover: ggmProver::BatchedProver,
    ot_receiver: Option<KosDeltaReceiver>,
    rng: AesRng,
    alpha_s: Vec<usize>,
    beta_s: Vec<RingT>,
    delta_s: Vec<RingT>,
    a_prime_s: Vec<RingT>,
    d_s: Vec<RingT>,
    chi_seed_s: Vec<Block>,
    x_star_s: Vec<RingT>,
    VP_s: Vec<RingT>,
    committed_VV_s: Vec<[u8; 32]>,
    is_init_done: bool,
    stats: BatchedProverStats,
}

#[derive(Copy, Clone, Debug, Default, Serialize)]
pub struct BatchedProverStats {
    pub stage_1_run_time: Duration,
    pub stage_2_run_time: Duration,
    pub stage_3_run_time: Duration,
    pub stage_4_run_time: Duration,
    pub stage_5_run_time: Duration,
    pub stage_6_run_time: Duration,
}

impl<RingT> BatchedProver<RingT>
where
    RingT: NewRing + Receivable,
    Standard: Distribution<RingT>,
    for<'a> &'a RingT: Sendable,
{
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
            stats: Default::default(),
        }
    }

    pub fn get_stats(&self) -> BatchedProverStats {
        self.stats
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

    pub fn stage_1_computation(&mut self, out_u: &mut [RingT], base_vole: (&[RingT], &[RingT])) {
        assert_eq!(out_u.len(), self.num_instances * self.output_size);
        debug_assert!(out_u.iter().all(|&x| x.is_zero()));
        assert_eq!(base_vole.0.len(), self.num_instances * 2);
        assert_eq!(base_vole.1.len(), self.num_instances * 2);
        for inst_i in 0..self.num_instances {
            self.alpha_s[inst_i] = self.rng.gen_range(0, self.output_size);
            self.chi_seed_s[inst_i] = self.rng.gen::<Block>();
            let a = base_vole.0[inst_i];
            let c = base_vole.1[inst_i];
            self.delta_s[inst_i] = c;
            while self.beta_s[inst_i].is_zero() {
                self.beta_s[inst_i] = self.rng.gen();
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

    pub fn stage_3_computation(&mut self, out_w: &mut [RingT]) {
        assert_eq!(out_w.len(), self.num_instances * self.output_size);
        self.ggm_prover.eval();
        (
            self.ggm_prover.get_output_blocks().par_iter(),
            out_w.par_iter_mut(),
        )
            .into_par_iter()
            .for_each(|(b_i, w_i)| {
                *w_i = RingT::from(*b_i);
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
        out_w: &mut [RingT],
        base_vole: (RingT, RingT),
        alpha: usize,
        delta: RingT,
        d: RingT,
        chi_seed: Block,
        x_star: &mut RingT,
        beta: RingT,
        VP: &mut RingT,
    ) {
        assert_eq!(out_w.len(), output_size);
        out_w[alpha] = RingT::ZERO; // we cannot assume that it is already zero
        let w_alpha: RingT = delta - d - out_w.iter().copied().sum();
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

        let x = base_vole.0;
        let z = base_vole.1;

        *x_star = if chi[alpha] { beta - x } else { -x };

        // TODO: apparently map is quite slow on large arrays -- is our use-case "large"?
        *VP = chi
            .iter()
            .zip(out_w.iter())
            .filter(|x| *x.0)
            .map(|x| x.1)
            .copied()
            .sum::<RingT>()
            - z;
    }

    #[allow(non_snake_case)]
    pub fn stage_5_computation(&mut self, out_w: &mut [RingT], base_vole: (&[RingT], &[RingT])) {
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
        let VV_s: Vec<RingT> = channel.receive_n(self.num_instances)?;
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
                    com.input(VV.as_ref());
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
        cache: &mut CachedProver<RingT>,
        alphas: &mut [usize],
        out_u: &mut [RingT],
        out_w: &mut [RingT],
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
        self.stats.stage_1_run_time = t_start.elapsed();
        let t_start = Instant::now();
        self.stage_2_communication(channel)?;
        self.stats.stage_2_run_time = t_start.elapsed();
        let t_start = Instant::now();
        self.stage_3_computation(out_w);
        self.stats.stage_3_run_time = t_start.elapsed();
        let t_start = Instant::now();
        self.stage_4_communication(channel)?;
        self.stats.stage_4_run_time = t_start.elapsed();
        let t_start = Instant::now();
        self.stage_5_computation(out_w, (&base_vole.0[..], &base_vole.1[..]));
        self.stats.stage_5_run_time = t_start.elapsed();
        let t_start = Instant::now();
        self.stage_6_communication(channel)?;
        self.stats.stage_6_run_time = t_start.elapsed();

        alphas.copy_from_slice(self.alpha_s.as_slice());
        Ok(())
    }
}
