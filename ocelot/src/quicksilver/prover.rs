use std::sync::mpsc::channel;
use std::time::{Duration, Instant};
use rand::distributions::{Distribution, Standard};
use rand::{Rng, SeedableRng};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rayon::prelude::{IntoParallelIterator, ParallelSliceMut};
use scuttlebutt::{AbstractChannel, AesRng, Block};
use scuttlebutt::channel::{Receivable, Sendable};
use scuttlebutt::ring::NewRing;
use crate::Error;
use crate::ot::mozzarella::{MozzarellaProver, MozzarellaProverStats};
use crate::ot::mozzarella::cache::prover::CachedProver;
use crate::ot::mozzarella::lpn::LLCode;
use serde::Serialize;


#[allow(non_snake_case)]
pub struct Prover<'a, RingT>
    where
        RingT: NewRing + Receivable,
        Standard: Distribution<RingT>,
        for<'b> &'b RingT: Sendable,
{
    mozProver: MozzarellaProver<'a, RingT>,
    stats: ProverStats,
}

#[derive(Copy, Clone, Debug, Default, Serialize)]
pub struct ProverStats {
    pub mozz_init: Duration,
    pub linear_comb_time: Duration,
    pub mozzarella_stats: MozzarellaProverStats,
}


#[allow(non_snake_case)]
impl <'a, RingT: NewRing> Prover<'a, RingT>
where
    RingT: NewRing + Receivable,
    Standard: Distribution<RingT>,
    for<'b> &'b RingT: Sendable,
{
    pub fn init<C: AbstractChannel>(
                    code: &'a LLCode<RingT>,
                    channel: &mut C,
                    cache: CachedProver<RingT>,
                    base_vole_len: usize,
                    num_sp_voles: usize,
                    sp_vole_len: usize,
    ) -> Self {

        let mut mozProver = MozzarellaProver::<RingT>::new(
            cache,
            &code,
            base_vole_len,
            num_sp_voles,
            sp_vole_len,
            false,
        );

        let mut stats: ProverStats = Default::default();

        let t_start = Instant::now();
        mozProver.init(channel).unwrap();
        stats.mozz_init = t_start.elapsed();

        // todo: Extend here for easier timing

        Self {
            mozProver,
            stats
        }
    }

    pub fn get_stats(&mut self) -> ProverStats {
        self.stats.mozzarella_stats = self.mozProver.get_stats();
        self.stats
    }

    pub fn get_run_time_init(&self) -> Duration {
        self.stats.mozz_init
    }

    // The mozVerifier already handles if there aren't any left, in which case it runs extend
    pub fn random<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(RingT, RingT), Error> {
        let (x, z)  = self.mozProver.vole(channel)?;
        return Ok((x,z))
    }

    pub fn input<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        x: RingT,
    ) -> Result<(RingT, RingT), Error>{
        //println!("I'm here");
        let (r,z) = self.random(channel)?;
        //println!("I'm here now");
        let y = x - r;
        channel.send(&y);

        Ok((x, z))
    }

    pub fn add (
        &mut self,
        (alpha, alpha_mac): (RingT, RingT),
        (beta, beta_mac): (RingT, RingT),
    ) -> Result<(RingT, RingT), Error> {

        Ok(((alpha + beta), ( alpha_mac + beta_mac)))
    }

    pub fn multiply<C: AbstractChannel> (
        &mut self,
        channel: &mut C,
        (alpha, alpha_mac): (RingT, RingT),
        (beta, beta_mac): (RingT, RingT),
    ) -> Result<((RingT, RingT),(RingT, RingT),(RingT, RingT)), Error> {
        let z = alpha * beta;
        let (z, z_mac) = self.input(channel, z)?;

        Ok(((alpha, alpha_mac), (beta, beta_mac), (z, z_mac)))
    }

    pub fn check_zero<C: AbstractChannel> () {
        // todo: C is supposed to be straight up public, so we need to subtract A*B from C and check
        //  if it's 0
    }

    pub fn check_multiply<C: AbstractChannel> (
        &mut self,
        channel: &mut C,
        triples: &mut [((RingT,RingT), (RingT,RingT), (RingT,RingT))],
        multi_thread: bool,
        chunk_size: usize,
    ) -> Result<(), Error>{

        let mut U = RingT::default();
        let mut V = RingT::default();

        let seed: Block = channel.receive().unwrap();
        let mut seeded_rng = AesRng::from_seed(seed);

        let check_time = Instant::now();

        if multi_thread {
            let t_start = Instant::now();

            let (U_out, V_out) = triples.par_chunks_exact_mut(chunk_size).map(|x| {
                let mut rng = AesRng::from_seed(Block::default());
                let (wl_U, wl_V): (RingT, RingT) = x.into_iter().fold((RingT::default(), RingT::default()),
                                                                      |(mut tmp_U, mut tmp_V), (alpha, beta, gamma)| {
                                                                          let chi = rng.gen::<RingT>();
                                                                          let u = ((alpha.1 * beta.1) * chi);
                                                                          let v = ((beta.0 * alpha.1) + (alpha.0 * beta.1) - gamma.1) * chi;
                                                                          (tmp_U + u, tmp_V + v)
                                                                      });
                (wl_U, wl_V)
            }).fold(|| (RingT::default(), RingT::default()), |(U, V), (tmp_u, tmp_v)| {
                (U + tmp_u, V + tmp_v)
            }).reduce(|| (RingT::default(), RingT::default()),
                      |(mut tmp_U, mut tmp_V), (alpha, beta)| {
                          let u = alpha;
                          let v = beta;
                          (tmp_U + u, tmp_V + v)
                      });

            println!("Time elapsed mul: {}", t_start.elapsed().as_millis());
            U = U_out;
            V = V_out;
        } else {
            let t_start = Instant::now();

            for cur in triples {
                let chi: RingT = seeded_rng.gen::<RingT>();

                // 0 is x (w), 1 is z (m)
                let w_alpha = cur.0.0;
                let m_alpha = cur.0.1;

                let w_beta = cur.1.0;
                let m_beta = cur.1.1;

                let w_gamma = cur.2.0;
                let m_gamma = cur.2.1;

                let a0i = m_alpha * m_beta;
                let a1i = (w_beta * m_alpha) + (w_alpha * m_beta) - m_gamma;

                U += (chi * a0i);
                V += (chi * a1i);
            }
            println!("Time elapsed mul: {}", t_start.elapsed().as_millis());

        }
        self.stats.linear_comb_time = check_time.elapsed();

        let (A1, A0) = self.random(channel)?;

        U += A0;
        V += A1;


        channel.send(&U);
        channel.send(&V);


        Ok(())
    }

}
