use std::cmp::Ordering;
use std::fmt;
use std::fmt::Formatter;
use std::ops::{AddAssign, MulAssign, SubAssign};
use crate::ring::Ring;


// remainder mod: x % n = ((x % n) + n) % n


#[derive(Clone, Hash)]
pub struct RX {
    val: u128,
    modulo: u128,
}

impl Copy for RX {}

impl Eq for RX {}

impl PartialEq<Self> for RX {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

impl AddAssign<Self> for RX {
    fn add_assign(&mut self, rhs: Self) {
        todo!()
    }
}

impl SubAssign<Self> for RX {
    fn sub_assign(&mut self, rhs: Self) {
        todo!()
    }
}

impl MulAssign<Self> for RX {
    fn mul_assign(&mut self, rhs: Self) {
        todo!()
    }
}

impl Ring for RX {
    fn as_mut_ptr(&mut self) -> *mut u8 {
        todo!()
    }

    fn as_ptr(&self) -> *const u8 {
        todo!()
    }
}