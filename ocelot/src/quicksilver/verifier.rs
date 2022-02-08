use crate::ot::mozzarella::cache::verifier::CachedVerifier;
use crate::ot::mozzarella::lpn::LLCode;
use crate::ot::mozzarella::{MozzarellaVerifier, MozzarellaVerifierStats};
use crate::Error;
use rand::distributions::{Distribution, Standard};
use rand::{CryptoRng, Rng, SeedableRng};
use rayon::prelude::*;
use scuttlebutt::channel::{Receivable, Sendable};
use scuttlebutt::ring::NewRing;
use scuttlebutt::{AbstractChannel, AesRng, Block};
use serde::Serialize;
use std::time::{Duration, Instant};

#[allow(non_snake_case)]
pub struct Verifier<'a, RingT>
where
    RingT: NewRing + Receivable,
    Standard: Distribution<RingT>,
    for<'b> &'b RingT: Sendable,
{
    mozVerifier: MozzarellaVerifier<'a, RingT>,
    delta: RingT,
    stats: VerifierStats,
}

#[derive(Copy, Clone, Debug, Default, Serialize)]
pub struct VerifierStats {
    pub mozz_init: Duration,
    pub linear_comb_time: Duration,
    pub mozzarella_stats: MozzarellaVerifierStats,
}

#[allow(non_snake_case)]
impl<'a, RingT: NewRing> Verifier<'a, RingT>
where
    RingT: NewRing + Receivable,
    Standard: Distribution<RingT>,
    for<'b> &'b RingT: Sendable,
{
    pub fn init<C: AbstractChannel>(
        delta: &mut RingT,
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

        let mut stats: VerifierStats = Default::default();
        stats.mozz_init = run_time_init;

        // todo: run extend here so we have enough and can count the time required to do so

        Self {
            mozVerifier,
            delta: *delta,
            stats,
        }
    }

    pub fn get_stats(&mut self) -> VerifierStats {
        // todo: also provide quicksilver stats
        self.stats.mozzarella_stats = self.mozVerifier.get_stats();
        self.stats
    }

    pub fn get_run_time_init(&self) -> Duration {
        self.stats.mozz_init
    }

    // The mozVerifier already handles if there aren't any left, in which case it runs extend
    pub fn random<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<RingT, Error> {
        let y = self.mozVerifier.vole(channel)?;
        return Ok(y);
    }

    pub fn random_batch<C: AbstractChannel>(&mut self, channel: &mut C, n: usize) -> Result<Vec<RingT>, Error> {
        self.mozVerifier.extend(channel, n)
    }

    pub fn input<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<RingT, Error> {
        // todo: ehm.
        let r = self.random(channel)?;
        let diff: RingT = channel.receive()?;
        let out = r - (diff * self.delta);
        Ok(out)
    }

    pub fn input_batch<C: AbstractChannel>(&mut self, channel: &mut C, n: usize) -> Result<Vec<RingT>, Error> {
        let mut out = self.random_batch(channel, n)?;
        let diff: Vec<RingT> = channel.receive_n(n)?;
        for i in 0..n {
            out[i] = out[i] - diff[i] * self.delta;
        }
        Ok(out)
    }

    pub fn add(&mut self, alpha: RingT, beta: RingT) -> Result<RingT, Error> {
        Ok(alpha + beta)
    }

    pub fn add_batch(&mut self, alpha: &[RingT], beta: &[RingT]) -> Vec<RingT> {
        assert_eq!(alpha.len(), beta.len());
        let n = alpha.len();
        let mut out = vec![RingT::default(); n];
        for i in 0..n {
            out[i] = alpha[i] + beta[i];
        }
        out
    }

    pub fn multiply<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        (alpha, beta): (RingT, RingT),
    ) -> Result<(RingT, RingT, RingT), Error> {
        let out = self.input(channel)?;
        Ok((alpha, beta, out))
    }

    pub fn multiply_batch<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        alpha: &[RingT],
        beta: &[RingT],
    ) -> Result<Vec<RingT>, Error> {
        assert_eq!(alpha.len(), beta.len());
        let n = alpha.len();
        self.input_batch(channel, n)
    }

    pub fn check_multiply<C: AbstractChannel, R: CryptoRng + Rng>(
        &mut self,
        channel: &mut C,
        mut rng: R,
        triples: &mut [(RingT, RingT, RingT)],
        multi_thread: bool,
        chunk_size: usize,
    ) -> Result<(), Error> {
        let mut W = RingT::default();

        let seed = rng.gen::<Block>();
        channel.send(&seed)?;
        let t_start = Instant::now();
        let mut seeded_rng = AesRng::from_seed(seed);
        println!("Time to create rng: {}", t_start.elapsed().as_millis());
        let check_start = Instant::now();

        if multi_thread {
            let t_start = Instant::now();

            println!("Sampling chis: {}", t_start.elapsed().as_millis());

            let t_start = Instant::now();
            //let mut tmp: Vec<(&RingT, &(RingT, RingT, RingT))> = chis.iter().zip(triples.iter()).collect();
            println!("Zipping: {}", t_start.elapsed().as_millis());
            let t_start = Instant::now();
            W = triples
                .par_chunks_exact_mut(chunk_size)
                .enumerate()
                .map(|(_idx, x)| {
                    // TODO: An initial seed should be sent from the verifier prior to this
                    let mut rng = AesRng::from_seed(Block::default());
                    x.into_iter()
                        .map(|y| rng.gen::<RingT>() * ((y.0 * y.1) + (y.2 * self.delta)))
                        .sum()
                })
                .sum();
            println!("Computing sum: {}", t_start.elapsed().as_millis());
        } else {
            let t_start = Instant::now();
            for (x, y, z) in triples.iter() {
                let chi = seeded_rng.gen::<RingT>();

                let bi = (*x) * (*y) + (*z * self.delta);

                W += bi * chi;
            }
            println!("Computing sum: {}", t_start.elapsed().as_millis());
        }
        let B = self.random(channel)?;
        W += B;

        self.stats.linear_comb_time = check_start.elapsed();

        let U: RingT = channel.receive()?;
        let V: RingT = channel.receive()?;

        let tmp = U - (V * self.delta);

        if W == tmp {
            println!("Check passed");
            Ok(())
        } else {
            println!("Someone lied");
            Err(Error::Other("checkMultiply fails".to_string()))
        }
    }
}
