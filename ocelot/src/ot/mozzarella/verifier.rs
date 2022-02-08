use crate::{
    ot::mozzarella::{
        cache::verifier::CachedVerifier,
        spvole::verifier::{
            BatchedVerifier as SpVerifier, BatchedVerifierStats as SpVerifierStats,
        },
        *,
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

pub struct Verifier<'a, RingT>
where
    RingT: Ring + Receivable,
    Standard: Distribution<RingT>,
    for<'b> &'b RingT: Sendable,
{
    spvole: SpVerifier<RingT>,
    base_vole_len: usize,
    sp_vole_total_len: usize,
    num_sp_voles: usize,
    cache: CachedVerifier<RingT>,
    code: &'a LLCode<RingT>,
    nightly_version: bool, // with extra protocol optimizations
    is_init_done: bool,
    stats: VerifierStats,
}

#[derive(Copy, Clone, Debug, Default, Serialize)]
pub struct VerifierStats {
    pub expansion_run_time: Duration,
    pub sp_stats: SpVerifierStats,
}

impl<'a, RingT> Verifier<'a, RingT>
where
    RingT: Ring + Receivable,
    Standard: Distribution<RingT>,
    for<'b> &'b RingT: Sendable,
{
    pub fn new_with_default_size(cache: CachedVerifier<RingT>, code: &'a LLCode<RingT>) -> Self {
        Self::new(cache, code, REG_MAIN_K, REG_MAIN_T, REG_MAIN_SPLEN, false)
    }

    pub fn new(
        cache: CachedVerifier<RingT>,
        code: &'a LLCode<RingT>,
        base_vole_len: usize,
        num_sp_voles: usize,
        sp_vole_len: usize,
        nightly_version: bool,
    ) -> Self {
        let spvole = SpVerifier::<RingT>::new(num_sp_voles, sp_vole_len, nightly_version);
        let sp_vole_total_len = sp_vole_len * num_sp_voles;
        assert_eq!(code.rows, base_vole_len);
        assert_eq!(code.columns, sp_vole_total_len);
        Self {
            spvole,
            base_vole_len,
            sp_vole_total_len,
            num_sp_voles,
            cache,
            code,
            nightly_version,
            is_init_done: false,
            stats: Default::default(),
        }
    }

    pub fn get_stats(&self) -> VerifierStats {
        self.stats
    }

    pub fn init<C: AbstractChannel>(&mut self, channel: &mut C, delta: RingT) -> Result<(), Error> {
        self.spvole.init(channel, delta)?;
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
        let y = self.base_extend(channel)?;

        // store voles in the cache
        self.cache.append(y.into_iter());
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

    pub fn vole<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<RingT, Error> {
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
    ) -> Result<Vec<RingT>, Error> {
        while !self.enough_voles_cached(n) {
            self.replenish_cache(channel)?;
        }
        Ok(self.cache.get(n))
    }

    pub fn base_extend<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<Vec<RingT>, Error> {
        assert!(self.is_init_done);

        let mut b = vec![Default::default(); self.sp_vole_total_len];
        self.spvole.extend(channel, &mut self.cache, &mut b)?;
        self.stats.sp_stats = self.spvole.get_stats();
        let k_cached: Vec<RingT> = self.cache.get(self.base_vole_len);
        let t_start = Instant::now();
        let out = self.code.mul_add(&k_cached[..], &b);
        self.stats.expansion_run_time = t_start.elapsed();

        return Ok(out);
    }
}
