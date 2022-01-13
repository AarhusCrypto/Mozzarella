use crate::{
    errors::Error,
    ot::{
        mozzarella::{cache::verifier::CachedVerifier, ggm::verifier as ggmVerifier},
        FixedKeyInitializer,
        KosDeltaSender,
    },
};
use rand::{rngs::OsRng, CryptoRng, Rng, SeedableRng};
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

        (out_v.par_iter_mut(), blocks.par_iter())
            .into_par_iter()
            .for_each(|(v, x)| {
                *v = R64::from(x.extract_0_u64());
            });
        (
            out_v.par_chunks_exact(self.output_size),
            self.d_s.par_iter_mut(),
        )
            .into_par_iter()
            .for_each(|(out_v, d)| {
                *d = -out_v.iter().sum::<R64>();
            });
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

    #[allow(non_snake_case)]
    pub fn stage_3_computation(&mut self) {
        let Delta = self.Delta;
        (
            self.gamma_s.par_iter_mut(),
            self.d_s.par_iter_mut(),
            self.b_s.par_iter(),
            self.a_prime_s.par_iter(),
        )
            .into_par_iter()
            .for_each(|(gamma, d, &b, &a_prime)| {
                *gamma = b - Delta * a_prime;
                *d += *gamma;
            });
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

    #[allow(non_snake_case)]
    pub fn stage_5_computation_helper(
        output_size: usize,
        out_v: &[R64],
        chi_seed: Block,
        VV: &mut R64,
    ) {
        assert_eq!(out_v.len(), output_size);
        // expand seed into bit vector chi
        // TODO: optimise to be "roughly" N/2
        let chi: Vec<bool> = {
            let mut indices = vec![false; output_size];
            let mut new_rng = AesRng::from_seed(chi_seed);

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
        *VV = chi
            .iter()
            .zip(out_v.iter())
            .filter(|x| *x.0)
            .map(|x| x.1)
            .sum::<R64>();
    }

    #[allow(non_snake_case)]
    pub fn stage_5_computation(&mut self, out_v: &[R64]) {
        assert_eq!(out_v.len(), self.num_instances * self.output_size);

        let output_size = self.output_size;
        (
            out_v.par_chunks_exact(self.output_size),
            self.chi_seed_s.par_iter(),
            self.VV_s.par_iter_mut(),
        )
            .into_par_iter()
            .for_each(|(out_v, &chi_seed, VV)| {
                Self::stage_5_computation_helper(output_size, out_v, chi_seed, VV);
            });
    }

    #[allow(non_snake_case)]
    pub fn stage_6_communication<C: AbstractChannel, RNG: Rng + CryptoRng>(
        &mut self,
        channel: &mut C,
        base_vole: &[R64],
        _rng: &mut RNG,
    ) -> Result<(), Error> {
        assert_eq!(base_vole.len(), 2 * self.num_instances);
        let x_star_s: Vec<R64> = channel.receive_n(self.num_instances)?;
        let y_star_s = &base_vole[self.num_instances..];
        let mut committed_VV_s = vec![[0u8; 32]; self.num_instances];

        let Delta = self.Delta;
        (
            self.VV_s.par_iter_mut(),
            x_star_s.par_iter(),
            y_star_s.par_iter(),
            self.commitment_randomness_s.par_iter_mut(),
            committed_VV_s.par_iter_mut(),
        )
            .into_par_iter()
            .for_each_init(
                || AesRng::new(),
                |rng, (VV, &x_star, &y_star, commitment_randomness, committed_VV)| {
                    let y: R64 = y_star - Delta * x_star;
                    *VV -= y;
                    *commitment_randomness = rng.gen();
                    *committed_VV = {
                        let mut com = ShaCommitment::new(*commitment_randomness);
                        com.input(&VV.0.to_le_bytes());
                        com.finish()
                    };
                },
            );

        channel.send(committed_VV_s.as_slice())?;
        channel.receive_into(self.VP_s.as_mut_slice())?;
        channel.send(self.VV_s.as_slice())?;
        channel.send(self.commitment_randomness_s.as_slice())?;

        if (self.VV_s.par_iter(), self.VP_s.par_iter())
            .into_par_iter()
            .all(|(VV, VP)| VV == VP)
        {
            Ok(())
        } else {
            Err(Error::EqCheckFailed)
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
