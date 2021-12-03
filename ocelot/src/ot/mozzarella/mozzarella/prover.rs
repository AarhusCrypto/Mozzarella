use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, Block};
use scuttlebutt::ring::R64;

use crate::Error;
// TODO: combine all of these use crate::ot
use crate::ot::{CorrelatedReceiver, RandomReceiver, Receiver as OtReceiver};
use crate::ot::mozzarella::spvole::prover::Prover as spsProver;
use crate::ot::mozzarella::utils::{flatten, flatten_mut};
use crate::ot::mozzarella::lpn::LLCode;

pub struct Prover{}

impl Prover {
    pub fn init() -> Self {
        Self{}
    }
    #[allow(non_snake_case)]
    pub fn extend<
        OT: OtReceiver<Msg=Block> + CorrelatedReceiver + RandomReceiver,
        C: AbstractChannel,
        R: Rng + CryptoRng,
        const K: usize,
        const N: usize,
        const T: usize,
        const D: usize,
        const LOG_SPLEN: usize,
        const SPLEN: usize,
    >(
        &mut self,
        code: &LLCode<K, N, D>,
        base_voles: &mut [(R64, R64)],
        spvole: &mut spsProver,
        rng: &mut R,
        channel: &mut C,
        alphas: &[usize; T], // error-positions of each spsvole
        ot_receiver: &mut OT,
    ) -> Result<(Vec<R64>, Vec<R64>), Error> {
        #[cfg(debug_assertions)]
            {
                debug_assert_eq!(T * SPLEN, N);
                for i in alphas.iter().copied() {
                    debug_assert!(i < SPLEN);
                }
            }
        let rep = 1;
        let num = 1;
        // have spsvole.extend run multiple executions
        let (mut w, u): (Vec<[R64; 16]>, Vec<[R64; 16]>) = spvole.extend(channel, rng, num, ot_receiver, base_voles, alphas)?;

        let e_flat = flatten::<R64, 16>(&u); // maybe works?
        let mut c_flat = flatten_mut::<16>(&mut w); // maybe works?
        let mut w_k: [R64; K] = [R64::default(); K];
        let mut u_k: [R64; K] = [R64::default(); K];
        for (idx, i) in base_voles.into_iter().enumerate() {
            u_k[idx] = i.0;
            w_k[idx] = i.1;
        }

        // compute x = A*u (and saves into c)
        let mut x = code.mul(&u_k);
        for (c, i) in x.chunks_exact_mut(SPLEN).zip(alphas.iter().copied()) {
            c[i] += e_flat[i];
        }

        // works?
        let out = code.mul_add(&w_k, c_flat);

        return Ok((x, out));
    }
}