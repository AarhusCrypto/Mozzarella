use super::*;
use crate::{
    ot::mozzarella::{cache::prover::CachedProver, spvole::prover::BatchedProver as SpProver},
    Error,
};
use rand::distributions::{Distribution, Standard};
use scuttlebutt::{
    channel::{Receivable, Sendable},
    ring::NewRing,
    AbstractChannel,
};
use std::time::Instant;

pub struct Prover<'a, RingT>
where
    RingT: NewRing + Receivable,
    Standard: Distribution<RingT>,
    for<'b> &'b RingT: Sendable,
{
    spvole: SpProver<RingT>,
    base_vole_len: usize,
    num_sp_voles: usize,
    sp_vole_single_len: usize,
    sp_vole_total_len: usize,
    cache: CachedProver<RingT>,
    code: &'a LLCode<RingT>,
    is_init_done: bool,
}

impl<'a, RingT> Prover<'a, RingT>
where
    RingT: NewRing + Receivable,
    Standard: Distribution<RingT>,
    for<'b> &'b RingT: Sendable,
{
    pub fn new_with_default_params(cache: CachedProver<RingT>, code: &'a LLCode<RingT>) -> Self {
        Self::new(cache, code, REG_MAIN_K, REG_MAIN_T, REG_MAIN_LOG_SPLEN)
    }

    pub fn new(
        cache: CachedProver<RingT>,
        code: &'a LLCode<RingT>,
        base_vole_len: usize,
        num_sp_voles: usize,
        log_sp_vole_single_len: usize,
    ) -> Self {
        let sp_vole_single_len = 1 << log_sp_vole_single_len;
        let spvole = SpProver::<RingT>::new(num_sp_voles, log_sp_vole_single_len);
        let sp_vole_total_len = sp_vole_single_len * num_sp_voles;
        assert_eq!(code.rows, base_vole_len);
        assert_eq!(code.columns, sp_vole_total_len);
        Self {
            spvole,
            base_vole_len,
            num_sp_voles,
            sp_vole_single_len,
            sp_vole_total_len,
            cache,
            code,
            is_init_done: false,
        }
    }

    pub fn init<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(), Error> {
        self.spvole.init(channel)?;
        self.is_init_done = true;
        Ok(())
    }

    pub fn vole<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(RingT, RingT), Error> {
        if self.cache.capacity() == REG_MAIN_VOLE {
            // replenish using main iteration
            let (x, z) = self.extend(channel)?;

            //dbg!("FILLING UP THE CACHE!");
            self.cache.append(x.into_iter(), z.into_iter());
        }

        let (x, z) = self.cache.pop();
        //println!("PROVER_OUTPUT:\t x={}, z={}", x,z);

        return Ok((x, z));
    }

    pub fn extend<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(Vec<RingT>, Vec<RingT>), Error> {
        assert!(self.is_init_done);

        // TODO: move allocations
        let mut c = vec![Default::default(); self.sp_vole_total_len];
        let mut e = vec![Default::default(); self.sp_vole_total_len];
        let mut alphas = vec![0; self.num_sp_voles];

        self.spvole
            .extend(channel, &mut self.cache, &mut alphas, &mut e, &mut c)?;

        let (u_old, w_old) = self.cache.get(self.base_vole_len);

        let start = Instant::now();
        // compute x = A*u (and saves into x)
        let mut x = self.code.mul(&u_old);
        println!("PROVER_EXPANSION_1: {:?}", start.elapsed());

        for (i, alpha_i) in alphas.iter().enumerate() {
            let index = i * self.sp_vole_single_len + alpha_i;
            x[index] += e[index];
        }

        let start = Instant::now();
        let z = self.code.mul_add(&w_old, &c);
        println!("PROVER_EXPANSION_2: {:?}", start.elapsed());

        return Ok((x, z));
    }
}
