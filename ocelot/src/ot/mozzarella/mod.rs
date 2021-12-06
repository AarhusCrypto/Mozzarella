use crate::ot::mozzarella::prover::Prover;
use crate::ot::mozzarella::verifier::Verifier;

pub mod mozzarella;
pub mod ggm;
pub mod spvole;
pub mod utils;
pub mod lpn;
mod prover;
mod verifier;

pub type MozzarellaProver = Prover;
pub type MozzarellaVerifier = Verifier;

//const REG_MAIN_K: usize = 589_760; // TODO: remove this eventually, when cache works
const REG_MAIN_K: usize = 0; // TODO: remove this eventually, when cache works
//const REG_MAIN_T: usize = 1_319; // TODO: remove this eventually, when cache works
const REG_MAIN_T: usize = 1; // TODO: remove this eventually, when cache works
//const REG_MAIN_N: usize = 10_805_248;
const REG_MAIN_N: usize = 16;
//const D: usize = 10;
const D: usize = 0;