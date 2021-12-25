use std::cmp::Ordering;
use std::fmt;
use std::fmt::Formatter;
use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};
use primitive_types::{U256, U512};
use crate::ring::Ring;


#[derive(Clone, Hash)]
pub struct RX(pub u128);


#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use crate::utils::{K_BIT_STRING, K_MODULO};


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
            ring: <u128>::from(*self),
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
        unsafe { std::mem::transmute(m) }
    }
}

impl From<u128> for RX {
    #[inline]
    fn from(inp: u128) -> Self {
        Self { 0: inp}
    }
}

impl From<RX> for u128 {
    #[inline]
    fn from(r: RX) -> u128 { unsafe { *(&r as *const _ as *const u128)} }
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
        self.0 = (self.0 + rhs.0) & K_BIT_STRING // we're guaranteed that the numbers are < 128 bits
    }
}

impl SubAssign<Self> for RX {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = (((self.0 as i128 - rhs.0 as i128) + (1 << K_MODULO)) as u128) & K_BIT_STRING
    }
}

impl MulAssign<Self> for RX {
    fn mul_assign(&mut self, rhs: Self) {
        self.0 = ((U256::from(self.0) * U256::from(self.0)) % U256::from(K_BIT_STRING)).as_u128()
    }
}

impl Mul for RX {
    type Output = RX;

    fn mul(self, rhs: Self) -> Self::Output {
        RX(((U256::from(self.0) * U256::from(self.0)) % U256::from(K_BIT_STRING)).as_u128())
    }
}

impl Add for RX {
    type Output = RX;

    fn add(self, rhs: Self) -> Self::Output {
        RX((self.0 + rhs.0) & K_BIT_STRING) // we're guaranteed that the numbers are < 128 bits
    }
}

impl Sub for RX {
    type Output = RX;

    fn sub(self, rhs: Self) -> Self::Output {
        RX((((self.0 as i128 - rhs.0 as i128) + (1 << K_MODULO)) as u128) & K_BIT_STRING)
    }
}

impl Ring for RX {
    fn as_mut_ptr(&mut self) -> *mut u8 {
        self.as_mut_ptr()
    }

    fn as_ptr(&self) -> *const u8 {
        self.as_ref().as_ptr()
    }
}

impl AsRef<[u8; 8]> for RX {
    #[inline]
    fn as_ref(&self) -> &[u8; 8] {
        unsafe { &*(self as *const RX as *const [u8; 8]) }
    }
}

// TODO: is U256 large enough? We'll be calling this sum on ~8000 elements, so they might overflow
//  and we might need U512 to handle this
impl std::iter::Sum for RX {
    fn sum<I: Iterator<Item=Self>>(iter: I) -> Self {
        let mut out: U512 = U512::zero();
        for e in iter {
            out += U512::from(e.0);
        }
        return RX(out.as_u128() &K_BIT_STRING)
    }
}

// TODO: is U256 large enough? We'll be calling this sum on ~8000 elements, so they might overflow
//  and we might need U512 to handle this
impl<'a> std::iter::Sum<&'a RX> for RX {
    fn sum<I>(iter: I) -> Self
        where
            I: Iterator<Item = &'a RX>,
    {
        let mut out: U512 = U512::zero();
        for e in iter {
            out += U512::from(e.0);
        }
        return RX(out.as_u128() &K_BIT_STRING)
    }
}

impl Default for RX {
    #[inline]
    fn default() -> Self {
        RX(0)
    }
}
