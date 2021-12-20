use std::cmp::Ordering;
use std::fmt;
use std::fmt::Formatter;
use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};
use crate::ring::Ring;


// remainder mod: x % n = ((x % n) + n) % n
// keep in mind, we work in 2k, so modulo is just truncating, so we just set the mod to be some
// power of two and truncate anything above that off by AND'ing with 2k 1's


#[derive(Clone, Hash)]
pub struct RX {
    /*
     TODO: Both of these should probably be in the mod file so they can be instantiated once and
        not every object needs to remember the u128 and such
     */
    val: u128,
    k: u8, // we just need to remember exponent k in 2^k
    k_bit_string: u128, // a bit string of k 1s 0b0001111...1

}



impl Copy for RX {}

impl Eq for RX {}


fn general_modulo(x: u128, modulo: u128) -> u128 {
    // TODO: This should just be truncation as well
    ((x % modulo) + modulo) % modulo
}

fn modulo_2k(x: u128, k_bit_string: u128) -> u128 {
    x & k_bit_string
}


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

impl Mul for RX {
    type Output = ();

    fn mul(self, rhs: Self) -> Self::Output {
        todo!()
    }
}

impl Add for RX {
    type Output = ();

    fn add(self, rhs: Self) -> Self::Output {
        todo!()
    }
}

impl Sub for RX {
    type Output = ();

    fn sub(self, rhs: Self) -> Self::Output {
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

