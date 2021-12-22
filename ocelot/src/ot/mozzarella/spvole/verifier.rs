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
use scuttlebutt::{ring::R64, F128};

#[allow(non_snake_case)]
pub struct SingleVerifier<'a, const N: usize, const H: usize> {
    ggm_verifier: ggmVerifier::Verifier,
    // rng: AesRng,
    index: usize,
    Delta: R64,
    base_vole: &'a Vec<R64>,
    out_v: &'a mut [R64; N],
    a_prime: R64,
    b: R64,
    gamma: R64,
    d: R64,
    chi_seed: Block,
    VV: R64,
    ggm_KKs: [(Block, Block); H],
    ggm_K_final: Block,
    ggm_checking_values: [Block; N],
    ggm_challenge_seed: Block,
    ggm_challenge_response: F128,
}

#[allow(non_snake_case)]
impl<'a, const N: usize, const H: usize> SingleVerifier<'a, N, H> {
    #[allow(non_snake_case)]
    pub fn init(
        index: usize,
        Delta: R64,
        base_vole: &'a Vec<R64>,
        out_v: &'a mut [R64; N],
    ) -> Self {
        Self {
            ggm_verifier: ggmVerifier::Verifier::init(),
            // rng: AesRng::new(),
            index,
            Delta,
            base_vole,
            out_v,
            a_prime: R64::default(),
            b: R64::default(),
            gamma: R64::default(),
            d: R64::default(),
            chi_seed: Block::default(),
            VV: R64::default(),
            ggm_KKs: [(Default::default(), Default::default()); H],
            ggm_K_final: Default::default(),
            ggm_checking_values: [Default::default(); N],
            ggm_challenge_seed: Default::default(),
            ggm_challenge_response: Default::default(),
        }
    }

    pub fn stage_1_computation(&mut self) {
        self.b = self.base_vole[2 * self.index];
        let t_start_ggm_gen = Instant::now();
        let (ggm_values, ggm_checking_values, ggm_K_final) =
            self.ggm_verifier.gen(&mut self.ggm_KKs).unwrap();
        println!("VERIFIER_GGM_GEN:\t {:?}", t_start_ggm_gen.elapsed());

        *self.out_v = ggm_values.map(|x| R64::from(x.extract_0_u64()));
        self.ggm_K_final = ggm_K_final;
        self.ggm_checking_values = ggm_checking_values;
    }

    pub fn stage_2_communication<
        C: AbstractChannel,
        OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender,
    >(
        &mut self,
        channel: &mut C,
        ot_sender: &mut OT,
    ) -> Result<(), Error> {
        self.a_prime = channel.receive()?;
        self.ggm_verifier.send::<_, _, N, H>(
            channel,
            ot_sender,
            &self.ggm_KKs,
            &self.ggm_K_final,
        )?;

        self.ggm_challenge_seed = self.ggm_verifier.receive_challenge(channel)?;
        Ok(())
    }

    pub fn stage_3_computation(&mut self) {
        self.gamma = self.b - self.Delta * self.a_prime;
        // compute d = gamma - \sum_{i \in [n]} v[i]
        self.d = self.gamma - self.out_v.iter().sum();
        self.ggm_challenge_response = self
            .ggm_verifier
            .compute_response::<N, H>(self.ggm_challenge_seed, &self.ggm_checking_values)
            .unwrap();
    }

    pub fn stage_4_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        self.ggm_verifier
            .send_response(channel, &self.ggm_challenge_response)?;
        channel.send(&self.d)?;
        self.chi_seed = channel.receive()?;
        Ok(())
    }

    pub fn stage_5_computation(&mut self) {
        // expand seed into bit vector chi
        // TODO: optimise to be "roughly" N/2
        let chi: [bool; N] = {
            let mut indices = [false; N];
            let mut new_rng = AesRng::from_seed(self.chi_seed);

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
        self.VV = chi
            .iter()
            .zip(self.out_v.iter())
            .filter(|x| *x.0)
            .map(|x| x.1)
            .sum::<R64>();
    }
    pub fn stage_6_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        let x_star: R64 = channel.receive()?;
        let y_star = self.base_vole[2 * self.index + 1];
        let y: R64 = y_star - self.Delta * x_star;
        self.VV -= y;

        // TODO: implement F_EQ functionality
        let VP = channel.receive()?;
        assert_eq!(self.VV, VP);

        Ok(())
    }

    #[allow(non_snake_case)]
    pub fn extend<
        C: AbstractChannel,
        OT: OtSender<Msg = Block> + CorrelatedSender + RandomSender,
    >(
        &mut self,
        channel: &mut C,
        ot_sender: &mut OT,
    ) -> Result<(), Error> {
        assert_eq!(1 << H, N);
        self.stage_1_computation();
        self.stage_2_communication(channel, ot_sender)?;
        self.stage_3_computation();
        self.stage_4_communication(channel)?;
        self.stage_5_computation();
        self.stage_6_communication(channel)?;

        Ok(())
    }
}

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
        _rng: &mut RNG,
        num: usize, // number of repetitions
        ot_sender: &mut OT,
        cache: &mut CachedVerifier,
    ) -> Result<Vec<[R64; N]>, Error> {
        assert_eq!(1 << H, N);

        // create result vector
        let mut out_v: Vec<[R64; N]> = Vec::with_capacity(num); // make stuff array as quicker
        unsafe { out_v.set_len(num) };

        let base_vole = cache.get(2 * num);

        for i in 0..num {
            let mut single_verifier =
                SingleVerifier::<N, H>::init(i, self.delta, &base_vole, &mut out_v[i]);
            single_verifier.extend(channel, ot_sender)?;
        }

        return Ok(out_v);
    }
}
