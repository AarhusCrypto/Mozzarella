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
use itertools::izip;
use rand::{CryptoRng, Rng, RngCore, SeedableRng};
use rayon::prelude::*;
use scuttlebutt::{ring::R64, AbstractChannel, AesRng, Block};
use std::convert::TryInto;

#[allow(non_snake_case)]
pub struct SingleProver {
    log_output_size: usize,
    output_size: usize,
    ggm_prover: ggmProver::Prover,
    rng: AesRng,
    index: usize,
    alpha: usize,
    beta: R64,
    delta: R64,
    a_prime: R64,
    d: R64,
    chi_seed: Block,
    x_star: R64,
    VP: R64,
}

#[allow(non_snake_case)]
impl SingleProver {
    pub fn new(index: usize, log_output_size: usize) -> Self {
        let output_size = 1 << log_output_size;
        Self {
            log_output_size,
            output_size,
            ggm_prover: ggmProver::Prover::new(log_output_size),
            rng: AesRng::new(),
            index,
            alpha: 0,
            beta: R64::default(),
            delta: R64::default(),
            a_prime: R64::default(),
            d: R64::default(),
            chi_seed: Block::default(),
            x_star: R64::default(),
            VP: R64::default(),
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

    pub fn stage_2_communication<
        C: AbstractChannel,
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
    >(
        &mut self,
        channel: &mut C,
        ot_receiver: &mut OT,
    ) -> Result<(), Error> {
        channel.send(&self.a_prime)?;
        self.ggm_prover.receive(channel, ot_receiver, self.alpha)?;
        self.ggm_prover.send_challenge(channel)?;
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

    pub fn stage_4_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        if !self.ggm_prover.receive_response_and_check(channel) {
            return Err(Error::Other("THE GAMMAS WERE NOT EQUAL!".to_string()));
        }
        self.d = channel.receive()?;
        channel.send(&self.chi_seed)?;
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

    pub fn stage_6_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        channel.send(&self.x_star)?;

        // TODO: implement F_EQ functionality
        channel.send(&self.VP)?;

        Ok(())
    }

    #[allow(non_snake_case)]
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
    log_sp_len: usize,
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
            .map(|i| SingleProver::new(i, log_sp_len))
            .collect();

        Self {
            num_sp_voles,
            log_sp_len,
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
            sp_i.stage_2_communication(channel, ot_receiver).unwrap();
        });
        self.single_provers
            .iter_mut()
            .zip(out_w.chunks_exact_mut(self.single_sp_len))
            .par_bridge()
            .for_each(|(sp_i, out_w_i)| {
                sp_i.stage_3_computation(out_w_i);
            });
        self.single_provers.iter_mut().for_each(|sp_i| {
            sp_i.stage_4_communication(channel).unwrap();
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
            sp_i.stage_6_communication(channel).unwrap();
        });

        for i in 0..self.num_sp_voles {
            alphas[i] = self.single_provers[i].get_alpha();
        }

        Ok(())
    }
}
