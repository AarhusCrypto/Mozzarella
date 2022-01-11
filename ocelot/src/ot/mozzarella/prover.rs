use super::*;
use crate::{
    ot::mozzarella::{cache::prover::CachedProver, spvole::prover::Prover as spProver},
    Error,
};
use scuttlebutt::{ring::R64, AbstractChannel};
use std::time::Instant;

pub struct Prover {
    spvole: spProver,
    base_vole_len: usize,
    num_sp_voles: usize,
    sp_vole_single_len: usize,
    sp_vole_total_len: usize,
    cache: CachedProver,
    is_init_done: bool,
}

impl Prover {
    pub fn new_with_default_params(cache: CachedProver) -> Self {
        Self::new(cache, REG_MAIN_K, REG_MAIN_T, REG_MAIN_LOG_SPLEN)
    }

    pub fn new(
        cache: CachedProver,
        base_vole_len: usize,
        num_sp_voles: usize,
        log_sp_vole_single_len: usize,
    ) -> Self {
        let sp_vole_single_len = 1 << log_sp_vole_single_len;
        let spvole = spProver::new(num_sp_voles, log_sp_vole_single_len);
        let sp_vole_total_len = sp_vole_single_len * num_sp_voles;
        Self {
            spvole,
            base_vole_len,
            num_sp_voles,
            sp_vole_single_len,
            sp_vole_total_len,
            cache,
            is_init_done: false,
        }
    }

    pub fn init<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(), Error> {
        self.spvole.init(channel)?;
        self.is_init_done = true;
        Ok(())
    }

    pub fn vole<C: AbstractChannel>(&mut self, channel: &mut C) -> Result<(R64, R64), Error> {
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
    ) -> Result<(Vec<R64>, Vec<R64>), Error> {
        assert!(self.is_init_done);
        // TODO: move to init?

        let code = &REG_MAIN_CODE;

        // TODO: move allocations
        let mut c = vec![Default::default(); self.sp_vole_total_len];
        let mut e = vec![Default::default(); self.sp_vole_total_len];
        let mut alphas = vec![0; self.num_sp_voles];

        self.spvole
            .extend(channel, &mut self.cache, &mut alphas, &mut e, &mut c)?;

        let (u_old, w_old) = self.cache.get(self.base_vole_len);

        let start = Instant::now();
        // compute x = A*u (and saves into x)
        let mut x = code.mul(&u_old);
        println!("PROVER_EXPANSION_1: {:?}", start.elapsed());

        for (i, alpha_i) in alphas.iter().enumerate() {
            let index = i * self.sp_vole_single_len + alpha_i;
            x[index] += e[index];
        }

        let start = Instant::now();
        let z = code.mul_add(&w_old, &c);
        println!("PROVER_EXPANSION_2: {:?}", start.elapsed());

        return Ok((x, z));
    }

    // same as extend, but using test code instead (TODO: make parameter)
    pub fn extend_test<C: AbstractChannel>(
        &mut self,
        channel: &mut C,
    ) -> Result<(Vec<R64>, Vec<R64>), Error> {
        assert!(self.is_init_done);
        // TODO: move to init?

        let code = &REG_TEST_CODE;

        // TODO: move allocations
        let mut c = vec![Default::default(); self.sp_vole_total_len];
        let mut e = vec![Default::default(); self.sp_vole_total_len];
        let mut alphas = vec![0; self.num_sp_voles];

        self.spvole
            .extend(channel, &mut self.cache, &mut alphas, &mut e, &mut c)?;

        let (u_old, w_old) = self.cache.get(self.base_vole_len);

        let start = Instant::now();
        // compute x = A*u (and saves into x)
        let mut x = code.mul(&u_old);
        println!("PROVER_EXPANSION_1: {:?}", start.elapsed());

        for (i, alpha_i) in alphas.iter().enumerate() {
            let index = i * self.sp_vole_single_len + alpha_i;
            x[index] += e[index];
        }

        let start = Instant::now();
        let z = code.mul_add(&w_old, &c);
        println!("PROVER_EXPANSION_2: {:?}", start.elapsed());

        return Ok((x, z));
    }
}
