use crate::{
    ot::mozzarella::{
        cache::verifier::CachedVerifier,
        spvole::verifier::Verifier as spVerifier,
        *,
    },
    Error,
};
use scuttlebutt::{ring::R64, AbstractChannel};
use std::time::Instant;

pub struct Verifier<'a> {
    spvole: spVerifier,
    base_vole_len: usize,
    sp_vole_total_len: usize,
    cache: CachedVerifier,
    code: &'a LLCode,
    is_init_done: bool,
}

impl<'a> Verifier<'a> {
    pub fn new_with_default_size(cache: CachedVerifier) -> Self {
        Self::new(
            cache,
            &REG_MAIN_CODE,
            REG_MAIN_K,
            REG_MAIN_T,
            REG_MAIN_LOG_SPLEN,
        )
    }

    pub fn new(
        cache: CachedVerifier,
        code: &'a LLCode,
        base_vole_len: usize,
        num_sp_voles: usize,
        log_sp_vole_len: usize,
    ) -> Self {
        let spvole = spVerifier::new(num_sp_voles, log_sp_vole_len);
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

    pub fn init<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
        ot_key: &[u8; 16],
    ) -> Result<(), Error> {
        self.spvole.init(channel, ot_key)?;
        self.is_init_done = true;
        Ok(())
    }

    pub fn vole<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<R64, Error> {
        // check if we have any saved in a cache
        if self.cache.capacity() == REG_MAIN_VOLE {
            // replenish using main iteration
            let y = self.extend(channel)?;

            self.cache.append(y.into_iter());
        }

        let out = self.cache.pop();
        return Ok(out);
    }

    pub fn extend<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<Vec<R64>, Error> {
        assert!(self.is_init_done);

        let mut b = vec![Default::default(); self.sp_vole_total_len];
        self.spvole.extend(channel, &mut self.cache, &mut b)?;

        let k_cached: Vec<R64> = self.cache.get(self.base_vole_len);

        let start = Instant::now();
        let out = self.code.mul_add(&k_cached[..], &b);
        println!("VERIFIER_EXPANSION: {:?}", start.elapsed());

        return Ok(out);
    }
}
