use std::collections::HashSet;
use std::iter::Sum;
use std::time::Instant;
use rand::{CryptoRng, Rng, SeedableRng};
use scuttlebutt::{AbstractChannel, AesRng, Block};
use scuttlebutt::ring::R64;
use crate::Error;
use crate::ot::mozzarella::ggm::prover as ggmProver;
use crate::ot::{CorrelatedReceiver, RandomReceiver, Receiver as OtReceiver};
use crate::ot::mozzarella::cache::prover::CachedProver;
use crate::ot::mozzarella::utils::unpack_bits;

pub struct Prover {}

impl Prover {

    pub fn init() -> Self {
        Self{}
    }

    #[allow(non_snake_case)]
    pub fn extend<
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
        C: AbstractChannel, RNG: CryptoRng + Rng, const N: usize, const H: usize>(
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


            let mut w: [R64;N] = [R64::default(); N];
            let (a, c): (R64, R64) = cache.pop();
            let delta: R64 = c;
            let mut beta: R64 = R64(rng.next_u64());
            loop {
                if beta.0 != 0 {
                    break;
                }
                beta = R64(rng.next_u64());
            }
            let mut a_prime = beta;
            a_prime -= a;
            channel.send(&a_prime).unwrap();

            let mut ggm_prover = ggmProver::Prover::init();
            let start = Instant::now();
            let (v, path_index) = ggm_prover.gen_eval(channel, ot_receiver, rng, &path)?;
            println!("PROVER_GGM_EVAL:\t {:?}", start.elapsed());


            let d: R64 = channel.receive()?;

            let mut w_alpha: R64 = delta;
            w_alpha -= d;
            w_alpha -= R64::sum(v.to_vec().into_iter()); // disgusting; can this be fixed to not require a vector?
            w = v;
            w[path_index.clone()] = w_alpha;


            //let mut indices = HashSet::new();
            let mut indices = [false; N];


            let seed: Block = rng.gen();
            let mut new_rng = AesRng::from_seed(seed);


            // N will always be even
            let mut i = 0;
            while i < N/2 {
                let tmp: usize = new_rng.gen_range(0, N);
                if indices[tmp] {
                    continue
                }
                indices[tmp] = true;
                i += 1;
            }

            /*
            // TODO: approximate rather than strictly require N/2
            while indices.len() < N / 2 {
                let tmp: usize = new_rng.gen_range(0, N);
                indices.insert(tmp);
            }
             */
            channel.send(&seed).unwrap();


            let copied_indices = indices.clone();
            let tmp = path_index.clone();
            let chi_alpha: R64 = R64(if copied_indices[tmp] { 1 } else { 0 });
            let (x,z): (R64, R64) = cache.pop();

            let mut x_star: R64 = beta;
            x_star *= chi_alpha;
            x_star -= x;

            channel.send(&x_star).unwrap();


            // TODO: apparently map is quite slow on large arrays -- is our use-case "large"?
            //let tmp_sum = indices.into_iter().map(|x| w[x as usize]);
            let tmp_sum = indices.iter().zip(w).filter(|x| *x.0).map(|x| x.1);



            let mut VP = R64::sum(tmp_sum.into_iter());
            VP -= z;

            channel.send(&VP).unwrap();

            let mut u: [R64; N] = [R64::default(); N];
            u[path_index] = beta;

            out_w.push(w);
            out_u.push(u);

        }

        return Ok((out_w, out_u));
    }
}