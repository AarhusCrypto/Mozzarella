mod r64;
mod rx;

pub use r64::R64;

use std::ops::{AddAssign, MulAssign, SubAssign};

pub trait Ring:
    'static
    + Clone
    + Copy
    + Eq
    + AddAssign<Self>
    + SubAssign<Self>
    + MulAssign<Self>
{

    fn as_mut_ptr(&mut self) -> *mut u8;

    fn as_ptr(&self) -> *const u8;

}
