use std::collections::HashSet;
use std::iter::Sum;
use rand::{CryptoRng, Rng, RngCore};
use scuttlebutt::{AbstractChannel, Block};
use scuttlebutt::ring::R64;
use crate::Error;
use crate::ot::mozzarella::ggm::receiver as ggmReceiver;
use crate::ot::{CorrelatedReceiver, RandomReceiver, Receiver as OtReceiver};
use crate::ot::mozzarella::spvole::generator::BiasedGen;

pub struct Prover {}

impl Prover {

    pub fn init() -> Self {
        Self{}
    }

    pub fn extend<
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
        C: AbstractChannel, RNG: CryptoRng + Rng>(
        &mut self,
        channel: &mut C,
        rng: &mut RNG,
        num: usize, // number of repetitions
        ot_receiver: &mut OT,
        base_voles: &mut Vec<(R64, R64)>,
    ) -> Result<Vec<Block>, Error> {
        println!("INFO:\tProver called!");

        const N: usize = 16;
        const H: usize = 4;

        let mut w: Vec<R64> = Vec::with_capacity(N);

        println!("BASE_VOLE:\t (prover) ({}, {})", base_voles[0].0, base_voles[0].1);

        let c: R64 = base_voles[0].1;
        let a: R64 = base_voles[0].0;
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
        println!("DEBUG:\t (prover) a_prime = beta - a1: {} = {} - {}", a_prime, beta, a);
        channel.send(&a_prime);
        println!("DEBUG:\t (prover) delta: {}", delta);
        println!("DEBUG:\t (prover) beta: {}", beta);


        let mut path: [bool; H] = [true, false, true, true];
        let mut ot_input: [bool; H] = [!path[0], !path[1], !path[2], !path[3]];
        println!("NOTICE_ME:\tProver still alive 1");
        let mut m: Vec<Block> = ot_receiver.receive(channel, &ot_input, rng)?;
        println!("NOTICE_ME:\tProver still alive 2");
        for i in &m {
            println!("INFO:\tm: {}", i);

        }

        let mut ggm_receiver = ggmReceiver::Receiver::init();
        let (v, path_index) = ggm_receiver.gen_eval(channel, rng, &mut path, &mut m)?;
        println!("NOTICE_ME:\tKEK1");

        for i in &v {
            println!("R64_OUT:\t {}", i);
        }

        let d:R64 = channel.receive()?;

        println!("DEBUG:\tProver received: {}", d);

        let mut w_alpha: R64 = delta;
        w_alpha -= d;
        w_alpha -= R64::sum(v.clone().into_iter());
        w = v;
        w[path_index.clone()] = w_alpha;



        // TODO: set seed, generate n/2 random numbers and use these as indices of where there should be a 1!
        // now the seed can be shared, instead of the vector :D Currently this uses something for blocks
        // which is likely more inefficient, as its F128 values



        let mut rng = rand::thread_rng();


        let mut indices = HashSet::new();

        // N will always be even
        while indices.len() < N/2 {
            let tmp: u16 = rng.gen_range(0,16);
            indices.insert(tmp);
        }

        for i in indices.clone() {
            println!("(prover):\t {}", i);
            channel.send(i);
        }

        let copied_indices = indices.clone();
        let tmp = path_index.clone();
        let chi_alpha: R64 = R64(if copied_indices.contains(&(tmp as u16)) {1} else {0});
        let x = base_voles[1].0;
        let z = base_voles[1].1;
        // is chi_alpha just the chi value at index alpha?
        let mut x_star: R64 = beta;
        x_star *= chi_alpha;
        x_star -= x;

        channel.send(&x_star);

        println!("PROVER:\t z={}", z);
        println!("PROVER:\t chi_alpha={}", chi_alpha);
        println!("PROVER:\t beta={}", beta);


        let tmp_sum = indices.into_iter().map(|x| w[x as usize]);

        let mut VP = R64::sum(tmp_sum.into_iter());
        VP -= z;

        println!("PROVER:\t VP={}", VP);

        // TODO: Mimix Feq
        // TODO: Output w and u

        return Ok(vec![Block::default()]);
    }
}