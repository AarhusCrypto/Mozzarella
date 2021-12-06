use std::collections::HashSet;
use std::iter::Sum;
use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, Block};
use scuttlebutt::ring::R64;
use crate::Error;
use crate::ot::mozzarella::ggm::receiver as ggmReceiver;
use crate::ot::{CorrelatedReceiver, RandomReceiver, Receiver as OtReceiver};
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
        base_voles: &mut [((R64, R64), (R64, R64))],
        alphas: &[usize],
    ) -> Result<(Vec<[R64; N]>, Vec<[R64; N]>), Error> {
        println!("INFO:\tProver called!");

        let mut out_w: Vec<[R64; N]> = Vec::with_capacity(num * N);
        let mut out_u: Vec<[R64; N]> = Vec::with_capacity(num * N); // can this also fit vector of arrays?

        for i in 0..num {
            // TODO: this gives me the final path index, so no need to compute it
            let alpha = alphas[i];

            let path: [bool; H] = unpack_bits::<H>(alpha);

            let ot_input: [bool; H] = path.map(|x| !x);

            let mut w: [R64;N] = [R64::default(); N];

            let c: R64 = base_voles[i].0.1;
            let a: R64 = base_voles[i].0.0;
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
            channel.send(&a_prime);

            let mut m: Vec<Block> = ot_receiver.receive(channel, &ot_input, rng)?;


            let mut ggm_receiver = ggmReceiver::Receiver::init();
            let (v, path_index) = ggm_receiver.gen_eval(channel, rng, &path, &mut m)?;

            for i in v {
                println!("PROVER_GGM:\t i={}", i);
            }

            let d: R64 = channel.receive()?;

            let mut w_alpha: R64 = delta;
            w_alpha -= d;
            w_alpha -= R64::sum(v.to_vec().into_iter()); // disgusting; can this be fixed to not require a vector?
            w = v;
            w[path_index.clone()] = w_alpha;


            // TODO: set seed, generate n/2 random numbers and use these as indices of where there should be a 1!
            // now the seed can be shared, instead of the vector :D
            let mut rng = rand::thread_rng();


            let mut indices = HashSet::new();

            // N will always be even
            while indices.len() < N / 2 {
                let tmp: usize = rng.gen_range(0, N);
                println!("PROVER_INDICES:\t i={}", tmp);
                indices.insert(tmp);
            }

            for i in indices.clone() {
                channel.send(i);
            }

            let copied_indices = indices.clone();
            let tmp = path_index.clone();
            let chi_alpha: R64 = R64(if copied_indices.contains(&tmp) { 1 } else { 0 });
            let x = base_voles[i].1.0;
            let z = base_voles[i].1.1;
            // is chi_alpha just the chi value at index alpha?
            let mut x_star: R64 = beta;
            x_star *= chi_alpha;
            x_star -= x;

            channel.send(&x_star);


            // TODO: apparently map is quite slow on large arrays -- is our use-case "large"?
            let tmp_sum = indices.into_iter().map(|x| w[x as usize]);

            let mut VP = R64::sum(tmp_sum.into_iter());
            VP -= z;

            println!("PROVER:\t VP={}", VP);
            channel.send(&VP);

            let mut u: [R64; N] = [R64::default(); N];
            u[path_index] = beta;

            out_w.push(w);
            out_u.push(u);

        }

        return Ok((out_w, out_u));
    }
}