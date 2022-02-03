use std::sync::mpsc::channel;
use std::time::{Duration, Instant};
use rand::{CryptoRng, Rng};
use rand::distributions::{Distribution, Standard};
use scuttlebutt::{AbstractChannel, Block};
use scuttlebutt::channel::{Receivable, Sendable};
use scuttlebutt::ring::NewRing;
use crate::Error;
use crate::ot::mozzarella::{MozzarellaVerifier, MozzarellaVerifierStats};
use crate::ot::mozzarella::cache::verifier::CachedVerifier;
use crate::ot::mozzarella::lpn::LLCode;

#[allow(non_snake_case)]
pub struct Verifier<'a, RingT>
where
    RingT: NewRing + Receivable,
    Standard: Distribution<RingT>,
    for<'b> &'b RingT: Sendable,
{
    mozVerifier: MozzarellaVerifier<'a, RingT>,
    delta: RingT,
    run_time_init: Duration,
}

#[allow(non_snake_case)]
impl <'a, RingT: NewRing> Verifier<'a, RingT>
    where
        RingT: NewRing + Receivable,
        Standard: Distribution<RingT>,
        for<'b> &'b RingT: Sendable,
{
    pub fn init<C: AbstractChannel>(delta: &mut RingT,
                code: &'a LLCode<RingT>,
                channel: &mut C,
                cache: CachedVerifier<RingT>,
                base_vole_len: usize,
                num_sp_voles: usize,
                sp_vole_len: usize,
    ) -> Self {

        let mut mozVerifier = MozzarellaVerifier::<RingT>::new(
            cache,
            &code,
            base_vole_len,
            num_sp_voles,
            sp_vole_len,
            false,
        );

        let t_start = Instant::now();
        mozVerifier.init(channel, *delta).unwrap();
        let run_time_init = t_start.elapsed();


       Self {
           mozVerifier,
           delta: *delta,
           run_time_init,

       }
    }

    pub fn get_stats(&self) -> MozzarellaVerifierStats {
        // todo: also provide quicksilver stats
        self.mozVerifier.get_stats()
    }

    pub fn get_run_time_init(&self) -> Duration {
        self.run_time_init
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

        let B = self.random(channel)?;

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