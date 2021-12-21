use crate::{
    ot::{
        mozzarella::{cache::prover::CachedProver, ggm::prover as ggmProver, utils::unpack_bits},
        CorrelatedReceiver,
        RandomReceiver,
        Receiver as OtReceiver,
    },
    Error,
};
use rand::{CryptoRng, Rng, SeedableRng};
use scuttlebutt::{ring::R64, AbstractChannel, AesRng, Block};
use std::time::Instant;

pub struct Prover {}

impl Prover {
    pub fn init() -> Self {
        Self {}
    }

    #[allow(non_snake_case)]
    pub fn extend<
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
        C: AbstractChannel,
        RNG: CryptoRng + Rng,
        const N: usize,
        const H: usize,
    >(
        &mut self,
        channel: &mut C,
        rng: &mut RNG,
        num: usize, // number of repetitions
        ot_receiver: &mut OT,
        cache: &mut CachedProver,
        alphas: &[usize],
    ) -> Result<(Vec<[R64; N]>, Vec<[R64; N]>), Error> {
        // Spawning these with_capicity requires too much space
        let mut out_w: Vec<[R64; N]> = Vec::new();
        let mut out_u: Vec<[R64; N]> = Vec::new();

        for i in 0..num {
            // TODO: this gives me the final path index, so no need to compute it
            let alpha = alphas[i];

            let path: [bool; H] = unpack_bits::<H>(alpha);

            let (a, c): (R64, R64) = cache.pop();
            let delta: R64 = c;
            let beta: R64 = {
                let mut beta = R64(rng.next_u64());
                while beta.0 == 0 {
                    beta = R64(rng.next_u64());
                }
                beta
            };
            let a_prime = beta - a;
            channel.send(&a_prime).unwrap();

            let mut ggm_prover = ggmProver::Prover::init();
            let start = Instant::now();
            let (v, _) = ggm_prover.gen_eval(channel, ot_receiver, &path)?;
            println!("PROVER_GGM_EVAL:\t {:?}", start.elapsed());

            let d: R64 = channel.receive()?;

            let w_alpha: R64 = delta - d - v.iter().sum();
            let mut w: [R64; N] = v;
            w[alpha] = w_alpha;

            // sample bit vector chi with Hamming weight N/2

            let seed: Block = rng.gen();
            channel.send(&seed).unwrap();

            let indices: [bool; N] = {
                let mut indices = [false; N];
                let mut new_rng = AesRng::from_seed(seed);

                // TODO: approximate rather than strictly require N/2
                // N will always be even
                let mut i = 0;
                while i < N / 2 {
                    let tmp: usize = new_rng.gen_range(0, N);
                    if indices[tmp] {
                        continue;
                    }
                    indices[tmp] = true;
                    i += 1;
                }
                indices
            };

            let chi_alpha: R64 = R64(if indices[alpha] { 1 } else { 0 });
            let (x, z): (R64, R64) = cache.pop();

            let x_star: R64 = chi_alpha * beta - x;

            channel.send(&x_star).unwrap();

            // TODO: apparently map is quite slow on large arrays -- is our use-case "large"?
            let VP: R64 = indices
                .iter()
                .zip(&w)
                .filter(|x| *x.0)
                .map(|x| x.1)
                .sum::<R64>()
                - z;

            // TODO: implement F_EQ functionality
            channel.send(&VP).unwrap();

            let mut u: [R64; N] = [R64::default(); N];
            u[alpha] = beta;

            out_w.push(w);
            out_u.push(u);
        }

        return Ok((out_w, out_u));
    }
}
