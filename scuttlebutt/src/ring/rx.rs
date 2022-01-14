use std::cmp::Ordering;
use std::convert::TryFrom;
use std::fmt;
use std::fmt::Formatter;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use primitive_types::{U256, U512};
use crate::ring::{NewRing, Ring};

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use crate::Block;
use crate::utils::{K_BIT_STRING, K_MODULO, STAT_SECURITY, STAT_SECURITY_STRING};

use rand::{
    distributions::{Distribution, Standard},
    Rng,
};


#[derive(Clone, Hash)]
pub struct RX(pub u128);


#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize)]
struct Helperr {
    pub ring: u128,
}

#[cfg(feature = "serde")]
impl Serialize for RX {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let helper = Helperr {
            ring: self.0 & K_BIT_STRING,
        };
        helper.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for RX {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        let helper = Helperr::deserialize(deserializer)?;
        Ok(RX::from(helper.ring.to_le_bytes()))
    }
}


impl From<[u8; 16]> for RX {
    #[inline]
    fn from(m: [u8; 16]) -> Self {
        let tmp: u128 = unsafe { std::mem::transmute(m) };
        RX::from(tmp & K_BIT_STRING)
    }
}

impl From<u128> for RX {
    #[inline]
    fn from(inp: u128) -> Self {
        Self { 0: inp & K_BIT_STRING}
    }
}

impl From<u64> for RX {
    #[inline]
    fn from(inp: u64) -> Self {
        Self { 0: inp as u128 & K_BIT_STRING}
    }
}

impl From<RX> for u128 {
    #[inline]
    fn from(r: RX) -> u128 {
        r.0
    }
}

impl From<Block> for RX {
    fn from(inp: Block) -> Self {
        Self {0: (inp.extract_u128() & K_BIT_STRING)}
    }
}

impl Ord for RX {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for RX {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.0))
    }
}


impl fmt::Display for RX {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}


impl Copy for RX {}

impl Eq for RX {}


impl PartialEq<Self> for RX {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl AddAssign<Self> for RX {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = ((U256::from(self.0) + U256::from(rhs.0)) & U256::from(K_BIT_STRING)).as_u128()

    }
}

impl SubAssign<Self> for RX {
    fn sub_assign(&mut self, rhs: Self) {
        let mut tmp = (self.0 as i128 - rhs.0 as i128);
        if tmp < 0 {
            tmp += (1 << K_MODULO)
        }
        self.0 = (tmp as u128) & K_BIT_STRING
    }
}

impl MulAssign<Self> for RX {
    fn mul_assign(&mut self, rhs: Self) {
        self.0 = ((U256::from(self.0) * U256::from(rhs.0)) & U256::from(K_BIT_STRING)).as_u128()
    }
}

impl Mul for RX {
    type Output = RX;

    fn mul(self, rhs: Self) -> Self::Output {
        RX::from(((U256::from(self.0) * U256::from(rhs.0)) & U256::from(K_BIT_STRING)).as_u128())
    }
}

impl Add for RX {
    type Output = RX;

    fn add(self, rhs: Self) -> Self::Output {
        RX::from(((U256::from(self.0) + U256::from(rhs.0)) & U256::from(K_BIT_STRING)).as_u128())
    }
}

impl Sub for RX {
    type Output = RX;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut tmp = (self.0 as i128 - rhs.0 as i128);
        if tmp < 0 {
            tmp += (1 << K_MODULO)
        }
        RX::from((tmp as u128) & K_BIT_STRING)
    }
}

impl Neg for RX {
    type Output = RX;
    fn neg(self) -> Self::Output {
        RX::default() - self
    }
}


impl Distribution<Self> for RX {
    #[inline]
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> RX {
        RX::from(rng.gen::<u128>())
    }
}

impl Ring for RX {

    fn as_mut_ptr(&mut self) -> *mut u8 {
        self.as_mut().as_mut_ptr()
    }

    fn as_ptr(&self) -> *const u8 {
        self.as_ref().as_ptr()
    }

    fn reduce_to_delta(b: Block) -> Self {
        Self {
            0: b.extract_u128() & STAT_SECURITY_STRING
        }
    }
}




/// The error which occurs if the inputted `u128` or bit pattern doesn't correspond to a field
/// element.
#[derive(Debug, Clone, Copy)]
pub struct BiggerThanModulus;
impl std::error::Error for BiggerThanModulus {}
impl std::fmt::Display for BiggerThanModulus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}


impl std::fmt::Debug for RX {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let val: u128 = (*self).into();
        write!(f, "{}", val)
    }
}


impl AsRef<[u8]> for RX {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        unsafe { &*(self as *const RX as *const [u8; 16]) }
    }
}

impl AsMut<[u8]> for RX {
    #[inline]
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe { &mut *(self as *mut RX as *mut [u8; 16]) }
    }
}

// TODO: is U256 large enough? We'll be calling this sum on ~8000 elements, so they might overflow
//  and we might need U512 to handle this -- Should probably be able to scale down if 512 is not needed
impl std::iter::Sum for RX {
    fn sum<I: Iterator<Item=Self>>(iter: I) -> Self {
        let mut out: U512 = U512::zero();
        for e in iter {
            out += U512::from(e.0);
        }
        return RX((out & U512::from(K_BIT_STRING)).as_u128())
    }
}



// TODO: is U256 large enough? We'll be calling this sum on ~8000 elements, so they might overflow
//  and we might need U512 to handle this -- Should probably be able to scale down if 512 is not needed
impl<'a> std::iter::Sum<&'a RX> for RX {
    fn sum<I>(iter: I) -> Self
        where
            I: Iterator<Item = &'a RX>,
    {
        let mut out: U512 = U512::zero();
        for e in iter {
            out += U512::from(e.0);
        }
        return RX((out & U512::from(K_BIT_STRING)).as_u128())
    }
}


impl Default for RX {
    #[inline]
    fn default() -> Self {
        RX(0)
    }
}

impl NewRing for RX {
    // TODO: fix values
    const BIT_LENGTH: usize = 64;
    const BYTE_LENGTH: usize = 8;
    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);
}

impl Distribution<RX> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> RX {
        RX(rng.gen::<u128>() & K_BIT_STRING)
    }
}
