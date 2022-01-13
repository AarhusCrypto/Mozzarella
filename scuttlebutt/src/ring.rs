mod r64;
pub(crate) mod rx;

pub use r64::R64;

use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};
use crate::Block;

pub trait Ring:
    'static
    + Clone
    + Copy
    + Eq
    + AddAssign<Self>
    + SubAssign<Self>
    + MulAssign<Self>
    + Mul
    + Add
    + Sub
{

    fn as_mut_ptr(&mut self) -> *mut u8;

    fn as_ptr(&self) -> *const u8;

    fn reduce_to_delta(b: Block) -> Self;

}
