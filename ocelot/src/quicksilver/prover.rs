use std::sync::mpsc::channel;
use std::time::{Duration, Instant};
use rand::distributions::{Distribution, Standard};
use scuttlebutt::{AbstractChannel, Block};
use scuttlebutt::channel::{Receivable, Sendable};
use scuttlebutt::ring::NewRing;
use crate::Error;
use crate::ot::mozzarella::{MozzarellaProver, MozzarellaProverStats};
use crate::ot::mozzarella::cache::prover::CachedProver;
use crate::ot::mozzarella::lpn::LLCode;

#[allow(non_snake_case)]
pub struct Prover<'a, RingT>
    where
        RingT: NewRing + Receivable,
        Standard: Distribution<RingT>,
        for<'b> &'b RingT: Sendable,
{
    mozProver: MozzarellaProver<'a, RingT>,
    run_time_init: Duration,
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

        let t_start = Instant::now();
        mozProver.init(channel).unwrap();
        let run_time_init = t_start.elapsed();


        Self {
            mozProver,
            run_time_init
        }
    }

    pub fn get_stats(&self) -> MozzarellaProverStats {
        self.mozProver.get_stats()
    }

    pub fn get_run_time_init(&self) -> Duration {
        self.run_time_init
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

        let (r,z) = self.random(channel)?;
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


    pub fn check_multiply<C: AbstractChannel> (
        &mut self,
        channel: &mut C,
        triples: &[((RingT,RingT), (RingT,RingT), (RingT,RingT))],
    ) -> Result<(), Error>{

        let mut U = RingT::default();
        let mut V = RingT::default();


        for cur in triples {
            let chi: RingT = channel.receive()?;

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

        let (A1, A0) = self.random(channel)?;

        U += A0;
        V += A1;

        channel.send(&U);
        channel.send(&V);

        Ok(())
    }

}