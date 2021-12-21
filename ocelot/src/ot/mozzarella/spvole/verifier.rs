use crate::{
    errors::Error,
    ot::{
        mozzarella::ggm::verifier as ggmVerifier,
        CorrelatedSender,
        RandomSender,
        Sender as OtSender,
    },
};
use rand::{CryptoRng, Rng, SeedableRng};
use scuttlebutt::{AbstractChannel, AesRng, Block};
use std::time::Instant;

use crate::ot::mozzarella::cache::verifier::CachedVerifier;
use scuttlebutt::ring::R64;

pub struct Verifier {
    pub delta: R64, // tmp
}

impl Verifier {
    pub fn init(delta: R64) -> Self {
        Self { delta }
    }
    #[allow(non_snake_case)]
    pub fn extend<
        OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender,
        C: AbstractChannel,
        RNG: CryptoRng + Rng,
        const N: usize,
        const H: usize,
    >(
        &mut self,
        channel: &mut C,
        rng: &mut RNG,
        num: usize, // number of repetitions
        ot_sender: &mut OT,
        cache: &mut CachedVerifier,
    ) -> Result<Vec<[R64; N]>, Error> {
        assert_eq!(1 << H, N);

        // create result vector
        let mut vs: Vec<[R64; N]> = Vec::with_capacity(num); // make stuff array as quicker
        unsafe { vs.set_len(num) };

        // generate the trees before, as we must now use OT to deliver the keys
        // this was not required in ferret, as they could mask the bits instead!
        for rep in 0..num {
            let b: R64 = cache.pop(); // some kind of error handling in case this cannot be done

            let a_prime: R64 = channel.receive()?;
            let gamma = b - self.delta * a_prime;

            let mut m: [(Block, Block); H] = [(Default::default(), Default::default()); H];

            // call the GGM sender and get the m and s
            let mut ggm_verifier = ggmVerifier::Verifier::init();

            let start = Instant::now();
            let s: [Block; N] = ggm_verifier.gen_tree(channel, ot_sender, &mut m)?;
            println!("VERIFIER_GGM_INIT:\t {:?}", start.elapsed());

            let ggm_out: [R64; N] = s.map(|x| R64::from(x.extract_0_u64()));

            // compute d = gamma - \sum_{i \in [n]} v[i]
            let d: R64 = gamma - ggm_out.iter().sum();

            channel.send(&d).unwrap();

            let y_star: R64 = cache.pop();

            let seed: Block = channel.receive().unwrap();
            // expand seed into bit vector chi
            // TODO: optimise to be "roughly" N/2
            let indices: [bool; N] = {
                let mut indices = [false; N];
                let mut new_rng = AesRng::from_seed(seed);

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

            let x_star: R64 = channel.receive()?;
            let y: R64 = y_star - self.delta * x_star;

            let VV: R64 = indices
                .iter()
                .zip(&ggm_out)
                .filter(|x| *x.0)
                .map(|x| x.1)
                .sum::<R64>()
                - y;

            // TODO: implement F_EQ functionality
            let VP = channel.receive()?;
            assert_eq!(VV, VP);

            vs[rep] = ggm_out;
        }

        return Ok(vs);
    }
}
