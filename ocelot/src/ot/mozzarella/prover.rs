use super::*;
use crate::{
    ot::mozzarella::{
        cache::prover::CachedProver,
        spvole::prover::{BatchedProver as SpProver, BatchedProverStats as SpProverStats},
    },
    Error,
};
use rand::distributions::{Distribution, Standard};
use scuttlebutt::{
    channel::{Receivable, Sendable},
    ring::Ring,
    AbstractChannel,
};
use serde::Serialize;
use std::time::{Duration, Instant};

pub struct Prover<'a, RingT>
where
    RingT: Ring + Receivable,
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
    nightly_version: bool, // with extra protocol optimizations
    is_init_done: bool,
    stats: ProverStats,
}

#[derive(Copy, Clone, Debug, Default, Serialize)]
pub struct ProverStats {
    pub expansion_1_run_time: Duration,
    pub expansion_2_run_time: Duration,
    pub sp_stats: SpProverStats,
}

impl<'a, RingT> Prover<'a, RingT>
where
    RingT: Ring + Receivable,
    Standard: Distribution<RingT>,
    for<'b> &'b RingT: Sendable,
{
    pub fn new_with_default_params(cache: CachedProver<RingT>, code: &'a LLCode<RingT>) -> Self {
        Self::new(cache, code, REG_MAIN_K, REG_MAIN_T, REG_MAIN_SPLEN, false)
    }

    pub fn new(
        cache: CachedProver<RingT>,
        code: &'a LLCode<RingT>,
        base_vole_len: usize,
        num_sp_voles: usize,
        sp_vole_single_len: usize,
        nightly_version: bool,
    ) -> Self {
        let spvole = SpProver::<RingT>::new(num_sp_voles, sp_vole_single_len, nightly_version);
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
            nightly_version,
            is_init_done: false,
            stats: Default::default(),
        }
    }

    pub fn get_stats(&self) -> ProverStats {
        self.stats
    }

    pub fn init<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(), Error> {
        self.spvole.init(channel)?;
        self.is_init_done = true;
        Ok(())
    }

    fn enough_voles_cached(&self, n: usize) -> bool {
        self.cache.capacity() >= n + reg_vole_required(self.base_vole_len, self.num_sp_voles)
    }

    fn replenish_cache<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(), Error> {
        if self.cache.capacity() < reg_vole_required(self.base_vole_len, self.num_sp_voles) {
            return Err(Error::Other("not enough base voles in cache".to_string()));
        }

        // replenish using main iteration
        let (x, z) = self.base_extend(channel)?;

        // store voles in the cache
        self.cache.append(x.into_iter(), z.into_iter());
        Ok(())
    }

    pub fn drain_cache(&mut self) {
        assert!(self.cache.capacity() >= reg_vole_required(self.base_vole_len, self.num_sp_voles));
        self.cache
            .get(self.cache.capacity() - reg_vole_required(self.base_vole_len, self.num_sp_voles));
        assert_eq!(
            self.cache.capacity(),
            reg_vole_required(self.base_vole_len, self.num_sp_voles)
        );
    }

    pub fn vole<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(RingT, RingT), Error> {
        if !self.enough_voles_cached(1) {
            self.replenish_cache(channel)?;
        }
        Ok(self.cache.pop())
    }

    pub fn ensure<C: AbstractChannel>(&mut self, channel: &mut C, n: usize) -> Result<(), Error> {
        while !self.enough_voles_cached(n) {
            self.replenish_cache(channel)?;
        }
        Ok(())
    }

    pub fn extend<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        n: usize,
    ) -> Result<(Vec<RingT>, Vec<RingT>), Error> {
        while !self.enough_voles_cached(n) {
            self.replenish_cache(channel)?;
        }
        Ok(self.cache.get(n))
    }

    pub fn base_extend<C: AbstractChannel>(
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
        self.stats.sp_stats = self.spvole.get_stats();

        let (u_old, w_old) = self.cache.get(self.base_vole_len);

        let t_start = Instant::now();
        // compute x = A*u (and saves into x)
        let mut x = self.code.mul(&u_old);
        self.stats.expansion_1_run_time = t_start.elapsed();

        for (i, alpha_i) in alphas.iter().enumerate() {
            let index = i * self.sp_vole_single_len + alpha_i;
            x[index] = (x[index] + e[index]).reduce();
        }

        let t_start = Instant::now();
        let z = self.code.mul_add(&w_old, &c);
        self.stats.expansion_2_run_time = t_start.elapsed();

        return Ok((x, z));
    }
}
