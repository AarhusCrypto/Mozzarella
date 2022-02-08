use crate::ot::mozzarella::cache::prover::CachedProver;
use crate::ot::mozzarella::lpn::LLCode;
use crate::ot::mozzarella::{MozzarellaProver, MozzarellaProverStats};
use crate::ot::mozzarella::utils::log2;
use crate::Error;
use rand::distributions::{Distribution, Standard};
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use scuttlebutt::channel::{Receivable, Sendable};
use scuttlebutt::ring::NewRing;
use scuttlebutt::{AbstractChannel, AesRng, Block};
use serde::Serialize;
use std::time::{Duration, Instant};

#[allow(non_snake_case)]
pub struct Prover<'a, RingT>
where
    RingT: NewRing + Receivable,
    Standard: Distribution<RingT>,
    for<'b> &'b RingT: Sendable,
{
    k: usize,
    statsec: usize,
    mozProver: MozzarellaProver<'a, RingT>,
    stats: ProverStats,
    is_init_done: bool,
}

#[derive(Copy, Clone, Debug, Default, Serialize)]
pub struct ProverStats {
    pub mozz_init: Duration,
    pub linear_comb_time: Duration,
    pub mozzarella_stats: MozzarellaProverStats,
}

#[allow(non_snake_case)]
impl<'a, RingT: NewRing> Prover<'a, RingT>
where
    RingT: NewRing + Receivable,
    Standard: Distribution<RingT>,
    for<'b> &'b RingT: Sendable,
{
    pub fn new(
        k: usize,
        statsec: usize,
        cache: CachedProver<RingT>,
        code: &'a LLCode<RingT>,
        base_vole_len: usize,
        num_sp_voles: usize,
        sp_vole_len: usize,
    ) -> Self {
        assert!(RingT::BIT_LENGTH >= k + 2 * statsec + log2(statsec));
        Self {
            k,
            statsec,
            mozProver: MozzarellaProver::<RingT>::new(
                cache,
                &code,
                base_vole_len,
                num_sp_voles,
                sp_vole_len,
                false,
            ),
            stats: Default::default(),
            is_init_done: false,
        }
    }

    pub fn init<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(), Error> {
        let t_start = Instant::now();
        self.mozProver.init(channel)?;
        self.stats.mozz_init = t_start.elapsed();
        self.is_init_done = true;
        Ok(())
    }

    pub fn apply_to_mozzarella_prover<ResT, F: FnOnce(&mut MozzarellaProver<RingT>) -> ResT>(
        &mut self,
        f: F,
    ) -> ResT {
        f(&mut self.mozProver)
    }

    pub fn get_stats(&mut self) -> ProverStats {
        self.stats.mozzarella_stats = self.mozProver.get_stats();
        self.stats
    }

    pub fn get_run_time_init(&self) -> Duration {
        self.stats.mozz_init
    }

    // The mozVerifier already handles if there aren't any left, in which case it runs extend
    pub fn random<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(RingT, RingT), Error> {
        let (x, z) = self.mozProver.vole(channel)?;
        return Ok((x, z));
    }

    pub fn random_batch<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        n: usize,
    ) -> Result<(Vec<RingT>, Vec<RingT>), Error> {
        self.mozProver.extend(channel, n)
    }

    pub fn input<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        x: RingT,
    ) -> Result<(RingT, RingT), Error> {
        //println!("I'm here");
        let (r, z) = self.random(channel)?;
        //println!("I'm here now");
        let y = x - r;
        channel.send(&y)?;

        Ok((x, z))
    }

    pub fn input_batch<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        inp: Vec<RingT>,
    ) -> Result<(Vec<RingT>, Vec<RingT>), Error> {
        let n = inp.len();
        let (mut r, r_mac) = self.random_batch(channel, n)?;
        for i in 0..n {
            r[i] = inp[i] - r[i];
        }
        channel.send(r.as_slice())?;
        Ok((inp, r_mac))
    }

    pub fn add(
        &mut self,
        (alpha, alpha_mac): (RingT, RingT),
        (beta, beta_mac): (RingT, RingT),
    ) -> Result<(RingT, RingT), Error> {
        Ok(((alpha + beta), (alpha_mac + beta_mac)))
    }

    pub fn add_batch(
        &mut self,
        (alpha, alpha_mac): (&[RingT], &[RingT]),
        (beta, beta_mac): (&[RingT], &[RingT]),
    ) -> (Vec<RingT>, Vec<RingT>) {
        let n = alpha.len();
        assert_eq!(alpha_mac.len(), n);
        assert_eq!(beta.len(), n);
        assert_eq!(beta_mac.len(), n);
        let mut out = vec![RingT::default(); n];
        let mut out_mac = vec![RingT::default(); n];
        for i in 0..n {
            out[i] = alpha[i] + beta[i];
            out_mac[i] = alpha_mac[i] + beta_mac[i];
        }
        (out, out_mac)
    }

    pub fn multiply<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        (alpha, alpha_mac): (RingT, RingT),
        (beta, beta_mac): (RingT, RingT),
    ) -> Result<((RingT, RingT), (RingT, RingT), (RingT, RingT)), Error> {
        let z = alpha * beta;
        let (z, z_mac) = self.input(channel, z)?;

        Ok(((alpha, alpha_mac), (beta, beta_mac), (z, z_mac)))
    }

    pub fn multiply_batch<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        (alpha, alpha_mac): (&[RingT], &[RingT]),
        (beta, beta_mac): (&[RingT], &[RingT]),
    ) -> Result<(Vec<RingT>, Vec<RingT>), Error> {
        let n = alpha.len();
        assert_eq!(alpha_mac.len(), n);
        assert_eq!(beta.len(), n);
        assert_eq!(beta_mac.len(), n);
        let mut out = vec![RingT::default(); n];
        for i in 0..n {
            out[i] = alpha[i] * beta[i];
        }
        self.input_batch(channel, out)
    }

    pub fn check_zero<C: AbstractChannel>() {
        // todo: C is supposed to be straight up public, so we need to subtract A*B from C and check
        //  if it's 0
    }

    pub fn check_multiply_batch<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        (alphas, alpha_macs): (&[RingT], &[RingT]),
        (betas, beta_macs): (&[RingT], &[RingT]),
        (gammas, gamma_macs): (&[RingT], &[RingT]),
        // multi_thread: bool,
        // chunk_size: usize,
    ) -> Result<(), Error> {
        let n = alphas.len();
        assert_eq!(n, betas.len());
        assert_eq!(n, gammas.len());
        assert_eq!(n, alpha_macs.len());
        assert_eq!(n, beta_macs.len());
        assert_eq!(n, gamma_macs.len());

        let mut U = RingT::ZERO;
        let mut V = RingT::ZERO;

        let chi_seed: Block = channel.receive().unwrap();
        let mut seeded_rng = AesRng::from_seed(chi_seed);

        let chis: Vec<RingT> = (0..n).map(|_| seeded_rng.gen()).collect();

        let t_start = Instant::now();

        for i in 0..n {
            let chi_i = chis[i];

            let w_alpha = alphas[i];
            let m_alpha = alpha_macs[i];

            let w_beta = betas[i];
            let m_beta = beta_macs[i];

            let m_gamma = gamma_macs[i];

            let a0i = m_alpha * m_beta;
            let a1i = (w_beta * m_alpha) + (w_alpha * m_beta) - m_gamma;

            U += chi_i * a0i;
            V += chi_i * a1i;
        }

        self.stats.linear_comb_time = t_start.elapsed();

        let (A1, A0) = self.random(channel)?;

        U += A0;
        V += A1;

        channel.send(&U)?;
        channel.send(&V)?;

        Ok(())
    }

    pub fn check_multiply<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        triples: &mut [((RingT, RingT), (RingT, RingT), (RingT, RingT))],
        multi_thread: bool,
        chunk_size: usize,
    ) -> Result<(), Error> {
        let mut U = RingT::default();
        let mut V = RingT::default();

        let seed: Block = channel.receive().unwrap();
        let mut seeded_rng = AesRng::from_seed(seed);

        let check_time = Instant::now();

        if multi_thread {
            let t_start = Instant::now();

            let (U_out, V_out) = triples
                .par_chunks_exact_mut(chunk_size)
                .map(|x| {
                    let mut rng = AesRng::from_seed(Block::default());
                    let (wl_U, wl_V): (RingT, RingT) = x.into_iter().fold(
                        (RingT::default(), RingT::default()),
                        |(tmp_U, tmp_V), (alpha, beta, gamma)| {
                            let chi = rng.gen::<RingT>();
                            let u = (alpha.1 * beta.1) * chi;
                            let v = ((beta.0 * alpha.1) + (alpha.0 * beta.1) - gamma.1) * chi;
                            (tmp_U + u, tmp_V + v)
                        },
                    );
                    (wl_U, wl_V)
                })
                .fold(
                    || (RingT::default(), RingT::default()),
                    |(U, V), (tmp_u, tmp_v)| (U + tmp_u, V + tmp_v),
                )
                .reduce(
                    || (RingT::default(), RingT::default()),
                    |(tmp_U, tmp_V), (alpha, beta)| {
                        let u = alpha;
                        let v = beta;
                        (tmp_U + u, tmp_V + v)
                    },
                );

            println!("Time elapsed mul: {}", t_start.elapsed().as_millis());
            U = U_out;
            V = V_out;
        } else {
            let t_start = Instant::now();

            for cur in triples {
                let chi: RingT = seeded_rng.gen::<RingT>();

                // 0 is x (w), 1 is z (m)
                let w_alpha = cur.0 .0;
                let m_alpha = cur.0 .1;

                let w_beta = cur.1 .0;
                let m_beta = cur.1 .1;

                // let w_gamma = cur.2.0;
                let m_gamma = cur.2 .1;

                let a0i = m_alpha * m_beta;
                let a1i = (w_beta * m_alpha) + (w_alpha * m_beta) - m_gamma;

                U += chi * a0i;
                V += chi * a1i;
            }
            println!("Time elapsed mul: {}", t_start.elapsed().as_millis());
        }
        self.stats.linear_comb_time = check_time.elapsed();

        let (A1, A0) = self.random(channel)?;

        U += A0;
        V += A1;

        channel.send(&U)?;
        channel.send(&V)?;

        Ok(())
    }
}
