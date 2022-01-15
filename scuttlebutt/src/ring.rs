pub mod z2r;
mod r64;
pub(crate) mod rx;

pub use r64::R64;
pub use rx::RX;
pub use z2r::Z2r;

use crate::Block;
use rand::distributions::{Distribution, Standard};
use std::{
    convert::From,
    fmt::Debug,
    iter::Sum,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

pub trait Ring:
    'static + Clone + Copy + Eq + AddAssign<Self> + SubAssign<Self> + MulAssign<Self> + Mul + Add + Sub
{
    fn as_mut_ptr(&mut self) -> *mut u8;

    fn as_ptr(&self) -> *const u8;

    fn reduce_to_delta(b: Block) -> Self;
}

pub trait NewRing:
    'static
    + Clone
    + Copy
    + Send
    + Sync
    + Default
    + Debug
    + Eq
    + AddAssign<Self>
    + SubAssign<Self>
    + MulAssign<Self>
    + Mul<Self, Output = Self>
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Neg<Output = Self>
    + Sum<Self>
    + From<Block>
    + AsRef<[u8]>
where
    Standard: Distribution<Self>,
{
    const BIT_LENGTH: usize;
    const BYTE_LENGTH: usize;

    const ZERO: Self;
    const ONE: Self;

    #[inline(always)]
    fn is_zero(&self) -> bool {
        *self == Self::ZERO
    }

    #[inline(always)]
    fn is_one(&self) -> bool {
        *self == Self::ONE
    }

    #[inline(always)]
    fn reduce(&self) -> Self { *self }

    #[inline(always)]
    fn is_reduced(&self) -> bool { true }

    fn reduce_to<const BITS: usize>(&self) -> Self;

    fn is_reduced_to<const BITS: usize>(&self) -> bool;

    // fn as_mut_ptr(&mut self) -> *mut u8;

    // fn as_ptr(&self) -> *const u8;
}
