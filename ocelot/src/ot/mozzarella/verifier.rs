use crate::{
    ot::mozzarella::{
        cache::verifier::CachedVerifier,
        spvole::verifier::BatchedVerifier as SpVerifier,
        *,
    },
    Error,
};
use rand::distributions::{Distribution, Standard};
use scuttlebutt::{
    channel::{Receivable, Sendable},
    ring::NewRing,
    AbstractChannel,
};
use std::time::Instant;

pub struct Verifier<'a, RingT>
where
    RingT: NewRing + Receivable,
    Standard: Distribution<RingT>,
    for<'b> &'b RingT: Sendable,
{
    spvole: SpVerifier<RingT>,
    base_vole_len: usize,
    sp_vole_total_len: usize,
    cache: CachedVerifier<RingT>,
    code: &'a LLCode<RingT>,
    is_init_done: bool,
}

impl<'a, RingT> Verifier<'a, RingT>
where
    RingT: NewRing + Receivable,
    Standard: Distribution<RingT>,
    for<'b> &'b RingT: Sendable,
{
    pub fn new_with_default_size(cache: CachedVerifier<RingT>, code: &'a LLCode<RingT>) -> Self {
        Self::new(cache, code, REG_MAIN_K, REG_MAIN_T, REG_MAIN_LOG_SPLEN)
    }

    pub fn new(
        cache: CachedVerifier<RingT>,
        code: &'a LLCode<RingT>,
        base_vole_len: usize,
        num_sp_voles: usize,
        log_sp_vole_len: usize,
    ) -> Self {
        let spvole = SpVerifier::<RingT>::new(num_sp_voles, log_sp_vole_len);
        let sp_vole_total_len = (1 << log_sp_vole_len) * num_sp_voles;
        assert_eq!(code.rows, base_vole_len);
        assert_eq!(code.columns, sp_vole_total_len);
        Self {
            spvole,
            base_vole_len,
            sp_vole_total_len,
            cache,
            code,
            is_init_done: false,
        }
    }

    pub fn init<C: AbstractChannel>(&mut self, channel: &mut C, delta: RingT) -> Result<(), Error> {
        self.spvole.init(channel, delta)?;
        self.is_init_done = true;
        Ok(())
    }

    pub fn vole<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<RingT, Error> {
        // check if we have any saved in a cache
        if self.cache.capacity() == REG_MAIN_VOLE {
            // replenish using main iteration
            let y = self.extend(channel)?;

            self.cache.append(y.into_iter());
        }

        let out = self.cache.pop();
        return Ok(out);
    }

    pub fn extend<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<Vec<RingT>, Error> {
        assert!(self.is_init_done);

        let mut b = vec![Default::default(); self.sp_vole_total_len];
        self.spvole.extend(channel, &mut self.cache, &mut b)?;

        let k_cached: Vec<RingT> = self.cache.get(self.base_vole_len);

        let start = Instant::now();
        let out = self.code.mul_add(&k_cached[..], &b);
        println!("VERIFIER_EXPANSION: {:?}", start.elapsed());

        return Ok(out);
    }
}
