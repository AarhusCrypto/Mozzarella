pub mod prover;
pub mod verifier;
mod mozzarella;


// main iteration parameters for regular error distribution
const REG_MAIN_K: usize = 589_760;
const REG_MAIN_N: usize = 10_805_248;
const REG_MAIN_T: usize = 1_319;
const REG_MAIN_LOG_SPLEN: usize = 13;
const REG_MAIN_SPLEN: usize = 1 << REG_MAIN_LOG_SPLEN;


lazy_static! {
    static ref REG_SETUP_CODE: LLCode::<REG_SETUP_K, REG_SETUP_N, CODE_D> =
        LLCode::from_seed(Block::default());
    static ref REG_MAIN_CODE: LLCode::<REG_MAIN_K, REG_MAIN_N, CODE_D> =
        LLCode::from_seed(Block::default());
}
