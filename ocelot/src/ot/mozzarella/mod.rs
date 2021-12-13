use crate::ot::mozzarella::prover::Prover;
use crate::ot::mozzarella::verifier::Verifier;

pub mod mozzarella;
pub mod ggm;
pub mod spvole;
pub mod utils;
pub mod lpn;
mod prover;
mod verifier;
pub mod cache;

pub type MozzarellaProver = Prover;
pub type MozzarellaVerifier = Verifier;

pub const fn reg_vole_required(k: usize, t: usize) -> usize {
    k + (t*2)
}

const CODE_D: usize = 10;

// main iteration parameters for regular error distribution
//pub const REG_MAIN_K: usize = 589_760; // TODO: remove this eventually, when cache works
pub const REG_MAIN_K: usize = 1118; // TODO: remove this eventually, when cache works
//pub const REG_MAIN_T: usize = 1_319; // TODO: remove this eventually, when cache works
pub const REG_MAIN_T: usize = 40; // TODO: remove this eventually, when cache works
//const REG_MAIN_N: usize = 10_805_248;
pub const REG_MAIN_N: usize = 20_480;
//pub const REG_MAIN_LOG_SPLEN: usize = 13;
//pub const REG_MAIN_SPLEN: usize = 1 << REG_MAIN_LOG_SPLEN;
pub const REG_MAIN_LOG_SPLEN: usize = 9;
pub const REG_MAIN_SPLEN: usize = 512;

pub const REG_MAIN_VOLE: usize = reg_vole_required(REG_MAIN_K, REG_MAIN_T);
