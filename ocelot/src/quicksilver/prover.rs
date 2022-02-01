use std::sync::mpsc::channel;
use rand::distributions::{Distribution, Standard};
use scuttlebutt::{AbstractChannel, Block};
use scuttlebutt::channel::{Receivable, Sendable};
use scuttlebutt::ring::NewRing;
use crate::Error;
use crate::ot::mozzarella::{MozzarellaProver};

#[allow(non_snake_case)]
pub struct Prover<'a, RingT>
    where
        RingT: NewRing + Receivable,
        Standard: Distribution<RingT>,
        for<'b> &'b RingT: Sendable,
{
    mozProver: MozzarellaProver<'a, RingT>,
}

#[allow(non_snake_case)]
impl <'a, RingT: NewRing> Prover<'a, RingT>
where
    RingT: NewRing + Receivable,
    Standard: Distribution<RingT>,
    for<'b> &'b RingT: Sendable,
{
    pub fn init(mozProver: MozzarellaProver<'a, RingT>) -> Self {
        Self {
            mozProver,
        }
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


    pub fn check_multiply<C: AbstractChannel> (
        &mut self,
        channel: &mut C,
        triples: &[((RingT,RingT), (RingT,RingT), (RingT,RingT))],
    ) -> Result<(), Error>{

        let mut U = RingT::default();
        let mut V = RingT::default();

        let chi: RingT = channel.receive()?;
        let mut power_chi = chi;


        for cur in triples {

            // 0 is x (w), 1 is z (m)
            let w_alpha = cur.0.0;
            let m_alpha = cur.0.1;

            let w_beta = cur.1.0;
            let m_beta = cur.1.1;

            let w_gamma = cur.2.0;
            let m_gamma = cur.2.1;

            let a0i = m_alpha * m_beta;
            let a1i = (w_beta * m_alpha) + (w_alpha * m_beta) - m_gamma;

            //println!("a0i: {}", a0i);
            //println!("a1i: {}", a1i);

            U += (power_chi * a0i);
            V += (power_chi * a1i);

            power_chi *= chi;
        }

        // todo: These are hardcoded, not sure how to generate them (B = A0 - A1 * Delta) while
        //      V doesn't learn A0, A1
        let A0 = RingT::from(Block::from(14429304277731815666));
        let A1 = RingT::from(Block::from(14681781395371891131));

        U += A0;
        V += A1;

        channel.send(&U);
        channel.send(&V);

        Ok(())
    }

}