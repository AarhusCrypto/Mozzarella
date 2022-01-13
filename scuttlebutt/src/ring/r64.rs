use std::fmt;
use std::fmt::Formatter;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use crate::ring::Ring;


#[derive(Clone, Hash)]
pub struct R64(pub u64);

impl Ring for R64 {

    /// Convert into a mutable pointer.
    #[inline]
    fn as_mut_ptr(&mut self) -> *mut u8 {
        self.as_mut().as_mut_ptr()
    }

    /// Convert into a pointer.
    #[inline]
    fn as_ptr(&self) -> *const u8 { self.as_ref().as_ptr()
    }
}

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize)]
struct Helperr {
    pub ring: u64,
}

#[cfg(feature = "serde")]
impl Serialize for R64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let helper = Helperr {
            ring: <u64>::from(*self),
        };
        helper.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for R64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        let helper = Helperr::deserialize(deserializer)?;
        Ok(R64::from(helper.ring.to_le_bytes()))
    }
}

impl std::fmt::Debug for R64 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let val: u64 = (*self).into();
        write!(f, "{}", val)
    }
}

impl From<R64> for u64 {
    #[inline]
    fn from(r: R64) -> u64 { unsafe { *(&r as *const _ as *const u64)} }
}

impl AsMut<[u8;8]> for R64 {
    #[inline]
    fn as_mut(&mut self) -> &mut [u8;8] {
        unsafe { &mut *(self as *mut R64 as *mut [u8; 8]) }
    }
}

impl AsRef<[u8; 8]> for R64 {
    #[inline]
    fn as_ref(&self) -> &[u8; 8] {
        unsafe { &*(self as *const R64 as *const [u8; 8]) }
    }
}

impl From<[u8; 8]> for R64 {
    #[inline]
    fn from(m: [u8; 8]) -> Self {
        unsafe { std::mem::transmute(m) }
    }
}


impl Ord for R64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for R64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.0))
    }
}


/*
impl From<R64> for u64 {
    #[inline]
    fn from(r: R64) -> u64 {
        r.0
    }
}
 */


impl From<u64> for R64 {
    #[inline]
    fn from(inp: u64) -> Self {
        Self { 0: inp}
    }
}


#[inline]
fn reduce(k: u128) -> u64 {
    (k % (1 << 64)) as u64
}

impl fmt::Display for R64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}


impl Copy for R64 {}

impl Eq for R64 {}

impl PartialEq<Self> for R64 {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl AddAssign<Self> for R64 {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_add((rhs.0))
    }
}

impl Add<Self> for R64 {
    type Output = R64;
    fn add(self, rhs: Self) -> Self::Output {
        R64(self.0.wrapping_add(rhs.0))
    }
}

impl SubAssign<Self> for R64 {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_sub(rhs.0)
    }
}

impl Sub<Self> for R64 {
    type Output = R64;
    fn sub(self, rhs: Self) -> Self::Output {
        R64(self.0.wrapping_sub(rhs.0))
    }
}


impl MulAssign<Self> for R64 {
    fn mul_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_mul(rhs.0)
    }
}

impl Mul<Self> for R64 {
    type Output = R64;
    fn mul(self, rhs: Self) -> Self::Output {
        R64(self.0.wrapping_mul(rhs.0))
    }
}

impl Neg for R64 {
    type Output = R64;
    fn neg(self) -> Self::Output {
        R64(self.0.wrapping_neg())
    }
}


impl std::iter::Sum for R64 {
    fn sum<I: Iterator<Item=Self>>(iter: I) -> Self {
        let mut out: u128 = 0;
        for e in iter {
            out += u128::from(e.0);
        }
        return R64(reduce(out))
    }
}

impl<'a> std::iter::Sum<&'a R64> for R64 {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a R64>,
    {
        let mut out: u128 = 0;
        for e in iter {
            out += u128::from(e.0);
        }
        return R64(reduce(out));
    }
}

impl Default for R64 {
    #[inline]
    fn default() -> Self {
        R64(0)
    }
}

