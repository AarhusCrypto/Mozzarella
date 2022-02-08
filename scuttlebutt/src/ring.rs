mod r64;
pub mod z2r;

pub use r64::R64;

use crate::Block;
use rand::distributions::{Distribution, Standard};
use std::fmt::Display;
use std::{
    convert::From,
    fmt::Debug,
    iter::Sum,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

pub trait Ring:
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
    + Mul<u64, Output = Self>
    + Add<Self, Output = Self>
    + Add<u64, Output = Self>
    + Sub<Self, Output = Self>
    + Neg<Output = Self>
    + Sum<Self>
    + From<Block>
    + AsRef<[u8]>
    + Display
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
    fn reduce(&self) -> Self {
        *self
    }

    #[inline(always)]
    fn is_reduced(&self) -> bool {
        true
    }

    fn reduce_to<const BITS: usize>(&self) -> Self;

    fn is_reduced_to<const BITS: usize>(&self) -> bool;

    fn reduce_to_32(&self) -> u32;

    fn reduce_to_64(&self) -> u64;

    fn sum(slice: &[Self]) -> Self {
        slice.iter().copied().sum()
    }

    // fn as_mut_ptr(&mut self) -> *mut u8;

    // fn as_ptr(&self) -> *const u8;
}
