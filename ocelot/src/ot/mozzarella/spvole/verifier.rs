use std::collections::HashSet;
use std::iter::Sum;
use crate::errors::Error;
use rand::{CryptoRng, Rng, SeedableRng};
use scuttlebutt::{AbstractChannel, AesRng, Block};
use crate::ot::mozzarella::ggm::verifier as ggmVerifier;
use crate::ot::{Sender as OtSender, RandomSender, CorrelatedSender};


use scuttlebutt::ring::R64;
use crate::ot::mozzarella::cache::verifier::CachedVerifier;

pub struct Verifier {
    pub delta: R64, // tmp
}

impl Verifier {
    pub fn init(delta: R64) -> Self {
        Self {
            delta,
        }
    }
    #[allow(non_snake_case)]
    pub fn extend<
        OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender,
        C: AbstractChannel, RNG: CryptoRng + Rng,
        const N: usize,
        const H: usize,
    >(
        &mut self,
        channel: &mut C,
        rng: &mut RNG,
        num: usize, // number of repetitions
        ot_sender: &mut OT,
        cache: &mut CachedVerifier,
    ) ->Result<Vec<[R64;N]>, Error> {
        //println!("H, N: {}, {}", H, N);
        assert_eq!(1 << H, N);
        //let base_vole = vec![1,2,3]; // tmp -- should come from some cache and be .. actual values

        //println!("BASE_VOLE:\t (verifier) {}",base_voles[0]);
        //println!("DELTA:\t (verifier) {}", self.delta);



        //println!("DEBUG:\t (verifier) gamma: {}", gamma);

        // create result vector
        let mut vs: Vec<[R64;N]> = Vec::with_capacity(num); // make stuff array as quicker
        unsafe { vs.set_len(num) };
        //let bs: Vec<usize> = channel.receive_n(num)?;
        //println!("INFO:\tReceiver called!");

        // generate the trees before, as we must now use OT to deliver the keys
        // this was not required in ferret, as they could mask the bits instead!
        for rep in 0..num {
            let b: R64 = cache.pop(); // some kind of error handling in case this cannot be done

            let a_prime: R64 = channel.receive()?;
            //println!("DEBUG:\t (verifier) a_prime: {}", a_prime);
            let mut gamma = b;
            let mut tmp = self.delta;
            tmp *= a_prime;
            gamma -= tmp;



            // used in the computation of "m"
            //let q = &cot[H * rep..H * (rep + 1)];

            let mut m: [(Block, Block); H] = [(Default::default(), Default::default()); H];
            //let mut s: [R64; N] = [Default::default(); N];

            // call the GGM sender and get the m and s
            let mut ggm_verifier = ggmVerifier::Verifier::init();

            //println!("INFO:\tGenerating GGM tree ...");
            let s: [Block; N] = ggm_verifier.gen_tree(channel, ot_sender, rng, &mut m)?;
            //println!("INFO:\tGenerated GGM tree");






            //let ggm_vec_out:Vec<R64> = s.iter().step_by(2).map(|x| R64::from(x.extract_0_u64())).collect();
            let ggm_out:[R64;N] = s.map(|x| R64::from(x.extract_0_u64()));

            // compute d = gamma - \sum_{i \in [n]} v[i]
            let mut d: R64 = gamma;


            d -= R64::sum(ggm_out.to_vec().into_iter()); // this sucks
            //println!("NOTICE_ME:\td={}", d);

            channel.send(&d).unwrap();

            let y_star: R64 = cache.pop();


            // TODO: optimise to be "roughly" N/2

            let mut indices = HashSet::new();
            let with_seed = true; // TODO: remove this testing stuff
            if with_seed {
                //let mut indices: Vec<usize> = Vec::new();
                let seed: Block = channel.receive().unwrap();
                let mut new_rng = AesRng::from_seed(seed);

                // N will always be even
                while indices.len() < N / 2 {
                    let tmp: usize = new_rng.gen_range(0, N);
                    //let tmp: usize = rng.gen_range(0, N);
                    //println!("PROVER_INDICES:\t i={}", tmp);
                    indices.insert(tmp);
                }
            } else {
                for _ in 0..N/2 {
                    indices.insert(channel.receive()?);
                }

                //for i in &indices {
                //    println!("(verifier):\t {}", i);
                //}
            }

            let x_star: R64 = channel.receive()?;
            let mut y: R64 = y_star;
            let mut tmp = self.delta;
            tmp *= x_star;
            y -= tmp;

            //println!("VERIFIER:\t y={}", y);
            //println!("VERIFIER:\t delta={}", self.delta);


            let tmp_sum = indices.into_iter().map(|x| ggm_out[x as usize]);

            let mut VV = R64::sum(tmp_sum.into_iter());
            VV -= y;

            //println!("VERIFIER:\t VV={}", VV);
            let VP = channel.receive()?;

            assert_eq!(VV, VP);

            vs[rep] = ggm_out;
        }

        return Ok(vs);

    }
}