use lazy_static::lazy_static;
use crate::ot::mozzarella::lpn::LLCode;
use scuttlebutt::Block;

pub mod prover;
pub mod verifier;
mod mozzarella;

const fn reg_cots_required(k: usize, t: usize, log_splen: usize) -> usize {
    k + log_splen * t + 128
}

const CODE_D: usize = 10;

// setup parameters for regular error distribution
const REG_SETUP_K: usize = 36_248;
// const REG_SETUP_N: usize = 609_728; Note: there is a typo in the paper!
const REG_SETUP_N: usize = 649_728;
const REG_SETUP_T: usize = 1_269;
const REG_SETUP_LOG_SPLEN: usize = 9;
const REG_SETUP_SPLEN: usize = 1 << REG_SETUP_LOG_SPLEN;
pub const REG_SETUP_COTS: usize = reg_cots_required(REG_SETUP_K, REG_SETUP_T, REG_SETUP_LOG_SPLEN);




// main iteration parameters for regular error distribution
//const REG_MAIN_K: usize = 589_760; // TODO: remove this eventually, when cache works
const REG_MAIN_K: usize = 0; // TODO: remove this eventually, when cache works
//const REG_MAIN_T: usize = 1_319; // TODO: remove this eventually, when cache works
const REG_MAIN_T: usize = 2; // TODO: remove this eventually, when cache works
//const REG_MAIN_N: usize = 10_805_248;
const REG_MAIN_N: usize = 32;
//const REG_MAIN_LOG_SPLEN: usize = 13;
//const REG_MAIN_SPLEN: usize = 1 << REG_MAIN_LOG_SPLEN;
const REG_MAIN_LOG_SPLEN: usize = 4;
const REG_MAIN_SPLEN: usize = 16;

pub const REG_MAIN_COTS: usize = reg_cots_required(REG_MAIN_K, REG_MAIN_T, REG_MAIN_LOG_SPLEN);


lazy_static! {
    static ref REG_SETUP_CODE: LLCode::<REG_SETUP_K, REG_SETUP_N, CODE_D> =
        LLCode::from_seed(Block::default());
    static ref REG_MAIN_CODE: LLCode::<REG_MAIN_K, REG_MAIN_N, CODE_D> =
        LLCode::from_seed(Block::default());
}
