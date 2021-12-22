use crate::{
    ot::{
        mozzarella::{cache::prover::CachedProver, ggm::prover as ggmProver, utils::unpack_bits},
        CorrelatedReceiver,
        RandomReceiver,
        Receiver as OtReceiver,
    },
    Error,
};
use rand::{CryptoRng, Rng, RngCore, SeedableRng};
use scuttlebutt::{ring::R64, AbstractChannel, AesRng, Block, F128};
use std::time::Instant;

#[allow(non_snake_case)]
pub struct SingleProver<'a, const N: usize, const H: usize> {
    ggm_prover: ggmProver::Prover,
    rng: AesRng,
    index: usize,
    alpha: usize,
    // bit decomposition of alpha
    alpha_bits: [bool; H],
    base_vole: &'a (Vec<R64>, Vec<R64>),
    out_w: &'a mut [R64; N],
    out_u: &'a mut [R64; N],
    beta: R64,
    delta: R64,
    a_prime: R64,
    d: R64,
    chi_seed: Block,
    x_star: R64,
    VP: R64,
    ggm_Ks: Vec<Block>,
    ggm_K_final: Block,
    ggm_checking_values: [Block; N],
    ggm_challenge_seed: Block,
    ggm_checking_hash: F128,
}

#[allow(non_snake_case)]
impl<'a, const N: usize, const H: usize> SingleProver<'a, N, H> {
    pub fn init(
        index: usize,
        alpha: usize,
        base_vole: &'a (Vec<R64>, Vec<R64>),
        out_w: &'a mut [R64; N],
        out_u: &'a mut [R64; N],
    ) -> Self {
        Self {
            ggm_prover: ggmProver::Prover::init(),
            rng: AesRng::new(),
            index,
            alpha,
            alpha_bits: unpack_bits::<H>(alpha),
            base_vole,
            out_w,
            out_u,
            beta: R64::default(),
            delta: R64::default(),
            a_prime: R64::default(),
            d: R64::default(),
            chi_seed: Block::default(),
            x_star: R64::default(),
            VP: R64::default(),
            ggm_Ks: Vec::new(),
            ggm_K_final: Block::default(),
            ggm_checking_values: [Block::default(); N],
            ggm_challenge_seed: Block::default(),
            ggm_checking_hash: F128::default(),
        }
    }

    pub fn stage_1_computation(&mut self) {
        self.chi_seed = self.rng.gen();
        let a = self.base_vole.0[2 * self.index];
        let c = self.base_vole.1[2 * self.index];
        self.delta = c;
        while self.beta.0 == 0 {
            self.beta = R64(self.rng.next_u64());
        }
        self.out_u[self.alpha] = self.beta;
        self.a_prime = self.beta - a;
    }

    pub fn stage_2_communication<
        C: AbstractChannel,
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
    >(
        &mut self,
        channel: &mut C,
        ot_receiver: &mut OT,
    ) -> Result<(), Error> {
        channel.send(&self.a_prime)?;
        // receive GGM tree
        let (mut Ks, K_final) =
            self.ggm_prover
                .receive::<C, OT, N, H>(channel, ot_receiver, &self.alpha_bits)?;
        std::mem::swap(&mut self.ggm_Ks, &mut Ks);
        self.ggm_K_final = K_final;
        self.ggm_challenge_seed = self.ggm_prover.send_challenge::<C>(channel)?;
        Ok(())
    }

    pub fn stage_3_computation(&mut self) {
        // evaluate GGM tree
        let t_start_ggm_eval = Instant::now();
        let (values, checking_values, _) = self
            .ggm_prover
            .eval::<N, H>(&self.alpha_bits, &self.ggm_Ks, self.ggm_K_final)
            .unwrap();
        println!("PROVER_GGM_EVAL:\t {:?}", t_start_ggm_eval.elapsed());
        // TODO: write directly into buffers
        *self.out_w = values;
        self.ggm_checking_values = checking_values;
        self.ggm_checking_hash = self
            .ggm_prover
            .compute_hash::<N, H>(self.ggm_challenge_seed, self.ggm_checking_values)
            .unwrap();
    }

    pub fn stage_4_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        if !self
            .ggm_prover
            .receive_response_and_check::<C>(channel, &self.ggm_checking_hash)
        {
            return Err(Error::Other("THE GAMMAS WERE NOT EQUAL!".to_string()));
        }
        self.d = channel.receive()?;
        channel.send(&self.chi_seed)?;
        Ok(())
    }

    pub fn stage_5_computation(&mut self) {
        let w_alpha: R64 = self.delta - self.d - self.out_w.iter().sum();
        self.out_w[self.alpha] = w_alpha;

        // expand seed to bit vector chi with Hamming weight N/2
        let chi: [bool; N] = {
            let mut indices = [false; N];
            let mut new_rng = AesRng::from_seed(self.chi_seed);

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

        let chi_alpha: R64 = R64(if chi[self.alpha] { 1 } else { 0 });
        let x = self.base_vole.0[2 * self.index + 1];
        let z = self.base_vole.1[2 * self.index + 1];

        self.x_star = chi_alpha * self.beta - x;

        // TODO: apparently map is quite slow on large arrays -- is our use-case "large"?
        self.VP = chi
            .iter()
            .zip(self.out_w.iter())
            .filter(|x| *x.0)
            .map(|x| x.1)
            .sum::<R64>()
            - z;
    }

    pub fn stage_6_communication<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(), Error> {
        channel.send(&self.x_star)?;

        // TODO: implement F_EQ functionality
        channel.send(&self.VP)?;

        Ok(())
    }

    #[allow(non_snake_case)]
    pub fn extend<
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
        C: AbstractChannel,
    >(
        &mut self,
        channel: &mut C,
        ot_receiver: &mut OT,
    ) -> Result<(), Error> {
        self.stage_1_computation();
        self.stage_2_communication(channel, ot_receiver)?;
        self.stage_3_computation();
        self.stage_4_communication(channel)?;
        self.stage_5_computation();
        self.stage_6_communication(channel)?;
        Ok(())
    }
}

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
        _rng: &mut RNG,
        num: usize, // number of repetitions
        ot_receiver: &mut OT,
        cache: &mut CachedProver,
        alphas: &[usize],
    ) -> Result<(Vec<[R64; N]>, Vec<[R64; N]>), Error> {
        let mut out_w: Vec<[R64; N]> = vec![[R64::default(); N]; num];
        let mut out_u: Vec<[R64; N]> = vec![[R64::default(); N]; num];

        let base_vole = cache.get(2 * num);

        for i in 0..num {
            let mut single_prover =
                SingleProver::<N, H>::init(i, alphas[i], &base_vole, &mut out_w[i], &mut out_u[i]);
            single_prover.extend(channel, ot_receiver)?;
        }

        return Ok((out_w, out_u));
    }
}
