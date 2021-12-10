use lazy_static::lazy_static;
use crate::ot::mozzarella::lpn::LLCode;
use scuttlebutt::Block;
use super::*;

pub mod prover;
pub mod verifier;


lazy_static! {
    static ref REG_MAIN_CODE: LLCode::<REG_MAIN_K, REG_MAIN_N, CODE_D> =
        LLCode::from_seed(Block::default());
}
