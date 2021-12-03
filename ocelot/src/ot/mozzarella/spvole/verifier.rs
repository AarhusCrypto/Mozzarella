use std::iter::Sum;
use crate::errors::Error;
use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, Block};
use crate::ot::mozzarella::ggm::sender as ggmSender;
use crate::ot::{Sender as OtSender, RandomSender, CorrelatedSender};


use scuttlebutt::ring::R64;

pub struct Verifier {
    pub delta: R64, // tmp
    l: usize, // repetitions of spvole
}

impl Verifier {
    pub fn init(delta: R64) -> Self {
        Self {
            delta,
            l: 0,
        }
    }
    #[allow(non_snake_case)]
    pub fn extend<
        OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender,
        C: AbstractChannel, RNG: CryptoRng + Rng>(
        &mut self,
        channel: &mut C,
        rng: &mut RNG,
        num: usize, // number of repetitions
        ot_sender: &mut OT,
        base_voles: &mut [R64],
    ) ->Result<Vec<[R64;16]>, Error> {
        const N: usize = 16; // tmp
        const H: usize = 4; //tmp
        assert_eq!(1 << H, N);
        //let base_vole = vec![1,2,3]; // tmp -- should come from some cache and be .. actual values

        //println!("BASE_VOLE:\t (verifier) {}",base_voles[0]);
        //println!("DELTA:\t (verifier) {}", self.delta);

        let b: R64 = base_voles[0];

        let a_prime: R64 = channel.receive()?;
        //println!("DEBUG:\t (verifier) a_prime: {}", a_prime);
        let mut gamma = b;
        let mut tmp = self.delta;
        tmp *= a_prime;
        gamma -= tmp;

        //println!("DEBUG:\t (verifier) gamma: {}", gamma);

        // create result vector
        let mut vs: Vec<[R64;N]> = Vec::with_capacity(num); // make stuff array as quicker
        unsafe { vs.set_len(num) };
        //let bs: Vec<usize> = channel.receive_n(num)?;
        println!("INFO:\tReceiver called!");

        // generate the trees before, as we must now use OT to deliver the keys
        // this was not required in ferret, as they could mask the bits instead!
        for rep in 0..num {
            // used in the computation of "m"
            //let q = &cot[H * rep..H * (rep + 1)];

            let mut m: [(Block, Block); H] = [(Default::default(), Default::default()); H];
            //let mut s: [R64; N] = [Default::default(); N];

            // call the GGM sender and get the m and s
            let mut ggm_sender = ggmSender::Sender::init();

            println!("INFO:\tGenerating GGM tree ...");
            let s: [Block; N] = ggm_sender.gen_tree(channel, rng, &mut m)?;
            println!("INFO:\tGenerated GGM tree");

            ot_sender.send(channel, &m, rng);


            let ggm_out:[R64;N] = s.map(|x| R64::from(x.extract_0_u64()));
            //for i in ggm_out {
            //    println!("NOTICE_ME:\t (Verifier) R64={}", i);
            //}
            // compute d = gamma - \sum_{i \in [n]} v[i]
            let mut d: R64 = gamma;


            d -= R64::sum(ggm_out.to_vec().into_iter()); // this sucks
            //println!("NOTICE_ME:\td={}", d);

            channel.send(&d);

            let y_star = base_voles[1];
            let indices: Vec<u16> = (0..N/2).map(|_| channel.receive().unwrap()).collect();

            //for i in &indices {
            //    println!("(verifier):\t {}", i);
            //}

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

            println!("VERIFIER:\t VV={}", VV);
            let VP = channel.receive()?;

            if VV == VP {
                println!("DEBUG:\tVV = VP!");
            } else {
                println!("DEBUG:\tPROVER CHEATED");
            }
            vs[rep] = ggm_out;
            // TODO: Mimic Feq -- probably just have prover send VP and check if VP == VV
            // TODO: output v (ggm_out)
        }

        return Ok(vs);

    }
}