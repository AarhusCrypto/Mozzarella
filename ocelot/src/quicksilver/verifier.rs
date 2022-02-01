use std::sync::mpsc::channel;
use rand::{CryptoRng, Rng};
use rand::distributions::{Distribution, Standard};
use scuttlebutt::{AbstractChannel, Block};
use scuttlebutt::channel::{Receivable, Sendable};
use scuttlebutt::ring::NewRing;
use crate::Error;
use crate::ot::mozzarella::{MozzarellaVerifier};

#[allow(non_snake_case)]
pub struct Verifier<'a, RingT>
where
    RingT: NewRing + Receivable,
    Standard: Distribution<RingT>,
    for<'b> &'b RingT: Sendable,
{
    mozVerifier: MozzarellaVerifier<'a, RingT>,
    delta: RingT,
}

#[allow(non_snake_case)]
impl <'a, RingT: NewRing> Verifier<'a, RingT>
    where
        RingT: NewRing + Receivable,
        Standard: Distribution<RingT>,
        for<'b> &'b RingT: Sendable,
{
    pub fn init(mozVerifier: MozzarellaVerifier<'a, RingT>, delta: RingT) -> Self {
       Self {
           mozVerifier,
           delta,
       }
    }

    // The mozVerifier already handles if there aren't any left, in which case it runs extend
    pub fn random<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<RingT, Error> {
        let y = self.mozVerifier.vole(channel)?;
        return Ok(y)
    }

    pub fn input<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<RingT, Error>{
        // todo: ehm.
        let r = self.random(channel)?;
        let diff: RingT = channel.receive()?;
        let out = r - (diff * self.delta);
        Ok(out)
    }

    pub fn add (
        &mut self,
        alpha: RingT,
        beta: RingT,
    ) -> Result<RingT, Error> {
        Ok(alpha + beta)
    }

    pub fn multiply<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        (alpha, beta): (RingT, RingT),
    ) -> Result<(RingT, RingT, RingT), Error> {
        let out = self.input(channel)?;
        Ok((alpha, beta, out))
    }

    pub fn check_multiply<C: AbstractChannel, R: CryptoRng + Rng> (
        &mut self,
        channel: &mut C,
        mut rng: R,
        triples: &[(RingT, RingT, RingT)],
    ) -> Result<(), Error>{




        let mut W = RingT::default();

        for  (x, y, z) in triples.iter() {
            let chi = rng.gen::<RingT>();
            channel.send(&chi);

            let bi = (*x) * (*y) + (*z * self.delta);

            W += (bi * chi);

        }

        // todo: This masking stuff is wrong. It's B = A0 - A1 * Delta (also, the identity is
        //      Key = Mac - x*Delta, but this means that V should only know B without A0 and A1?
        let A0 = RingT::from(Block::from(14429304277731815666));
        let A1 = RingT::from(Block::from(14681781395371891131));
        let B = A0 - (A1 * self.delta);

        W += B;

        let U: RingT = channel.receive()?;
        let V: RingT = channel.receive()?;

        let tmp = (U - (V * self.delta));
        if W == tmp {
            println!("Check passed");
            Ok(())
        } else {
            println!("Someone lied");
            Err(Error::Other("checkMultiply fails".to_string()))
        }
    }

}