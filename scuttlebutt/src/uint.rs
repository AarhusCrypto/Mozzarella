// MIT License
//
// Copyright (c) 2022 Lennart Braun
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crunchy::unroll;
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use std::cmp;
use std::convert;
use std::fmt;
use std::ops;

#[derive(Debug, Copy, Clone)]
pub struct U192(pub [u64; 3]);

#[derive(Debug, Copy, Clone)]
pub struct U256(pub [u64; 4]);

impl U192 {
    pub const ZERO: U192 = U192([0, 0, 0]);
    pub const ONE: U192 = U192([1, 0, 0]);
    pub const MIN: U192 = Self::ZERO;
    pub const MAX: U192 = U192([0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff]);

    #[inline(always)]
    pub fn is_zero(&self) -> bool {
        *self == Self::ZERO
    }

    #[inline(always)]
    pub fn is_one(&self) -> bool {
        *self == Self::ONE
    }

    #[inline(always)]
    pub fn sum(slice: &[Self]) -> Self {
        let mut s = Self::ZERO;
        for x in slice {
            s = s + *x;
        }
        s
    }
}

impl U256 {
    pub const ZERO: U256 = U256([0, 0, 0, 0]);
    pub const ONE: U256 = U256([1, 0, 0, 0]);
    pub const MIN: U256 = Self::ZERO;
    pub const MAX: U256 = U256([
        0xffffffffffffffff,
        0xffffffffffffffff,
        0xffffffffffffffff,
        0xffffffffffffffff,
    ]);

    #[inline(always)]
    pub fn is_zero(&self) -> bool {
        *self == Self::ZERO
    }

    #[inline(always)]
    pub fn is_one(&self) -> bool {
        *self == Self::ONE
    }

    #[inline(always)]
    pub fn sum(slice: &[Self]) -> Self {
        let mut s = Self::ZERO;
        for x in slice {
            s = s + *x;
        }
        s
    }
}

impl convert::From<u128> for U192 {
    #[inline(always)]
    fn from(x: u128) -> Self {
        U192([(x & 0xffffffffffffffff) as u64, (x >> 64) as u64, 0])
    }
}

impl convert::From<u128> for U256 {
    #[inline(always)]
    fn from(x: u128) -> Self {
        U256([(x & 0xffffffffffffffff) as u64, (x >> 64) as u64, 0, 0])
    }
}

impl convert::From<(u128, u128)> for U256 {
    #[inline(always)]
    fn from(x: (u128, u128)) -> Self {
        let (x0, x1) = x;
        U256([
            (x0 & 0xffffffffffffffff) as u64,
            (x0 >> 64) as u64,
            (x1 & 0xffffffffffffffff) as u64,
            (x1 >> 64) as u64,
        ])
    }
}

impl cmp::PartialEq<U192> for U192 {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl cmp::Eq for U192 {}

impl cmp::PartialEq<U256> for U256 {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl cmp::Eq for U256 {}

impl ops::Add<U192> for U192 {
    type Output = Self;

    #[inline(always)]
    #[allow(unused_assignments)]
    fn add(self, other: Self) -> Self {
        let u = &self.0;
        let v = &other.0;
        let mut w = [0u64; 3];
        let mut carry = false;
        unroll! {
        for i in 0..3 {
            let (tmp, o1) = u[i].overflowing_add(v[i]);
            let (tmp, o2) = tmp.overflowing_add(carry as u64);
            w[i] = tmp;
            carry = o1 || o2;
        }
        }
        Self(w)
    }
}

impl ops::Add<U256> for U256 {
    type Output = Self;

    #[inline(always)]
    #[allow(unused_assignments)]
    fn add(self, other: Self) -> Self {
        let u = &self.0;
        let v = &other.0;
        let mut w = [0u64; 4];
        let mut carry = false;
        unroll! {
        for i in 0..4 {
            let (tmp, o1) = u[i].overflowing_add(v[i]);
            let (tmp, o2) = tmp.overflowing_add(carry as u64);
            w[i] = tmp;
            carry = o1 || o2;
        }
        }
        Self(w)
    }
}

impl ops::Add<u64> for U192 {
    type Output = Self;

    #[inline(always)]
    #[allow(unused_assignments)]
    fn add(self, other: u64) -> Self {
        let u = &self.0;
        let mut w = [0u64; 3];

        let (tmp, o) = u[0].overflowing_add(other);
        w[0] = tmp;
        let (tmp, o) = u[1].overflowing_add(o as u64);
        w[1] = tmp;
        let (tmp, _) = u[2].overflowing_add(o as u64);
        w[2] = tmp;
        Self(w)
    }
}

impl ops::Add<u64> for U256 {
    type Output = Self;

    #[inline(always)]
    #[allow(unused_assignments)]
    fn add(self, other: u64) -> Self {
        let u = &self.0;
        let mut w = [0u64; 4];

        let (tmp, o) = u[0].overflowing_add(other);
        w[0] = tmp;
        let (tmp, o) = u[1].overflowing_add(o as u64);
        w[1] = tmp;
        let (tmp, o) = u[2].overflowing_add(o as u64);
        w[2] = tmp;
        let (tmp, _) = u[3].overflowing_add(o as u64);
        w[3] = tmp;
        Self(w)
    }
}

impl ops::AddAssign<U192> for U192 {
    #[inline(always)]
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl ops::AddAssign<U256> for U256 {
    #[inline(always)]
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl ops::Sub<U192> for U192 {
    type Output = Self;

    #[inline(always)]
    #[allow(unused_assignments)]
    fn sub(self, other: Self) -> Self {
        let u = &self.0;
        let v = &other.0;
        let mut w = [0u64; 3];
        let mut carry = false;
        unroll! {
        for i in 0..3 {
            let (tmp, o1) = u[i].overflowing_sub(v[i]);
            let (tmp, o2) = tmp.overflowing_sub(carry as u64);
            w[i] = tmp;
            carry = o1 || o2;
        }
        }
        Self(w)
    }
}

impl ops::Sub<U256> for U256 {
    type Output = Self;

    #[inline(always)]
    #[allow(unused_assignments)]
    fn sub(self, other: Self) -> Self {
        let u = &self.0;
        let v = &other.0;
        let mut w = [0u64; 4];
        let mut carry = false;
        unroll! {
        for i in 0..4 {
            let (tmp, o1) = u[i].overflowing_sub(v[i]);
            let (tmp, o2) = tmp.overflowing_sub(carry as u64);
            w[i] = tmp;
            carry = o1 || o2;
        }
        }
        Self(w)
    }
}

impl ops::SubAssign<U192> for U192 {
    #[inline(always)]
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl ops::SubAssign<U256> for U256 {
    #[inline(always)]
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl ops::Neg for U192 {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self {
        Self::ZERO - self
    }
}

impl ops::Neg for U256 {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self {
        Self::ZERO - self
    }
}

impl ops::Mul<U192> for U192 {
    type Output = Self;

    #[inline(always)]
    #[allow(unused_assignments)]
    fn mul(self, other: Self) -> Self {
        let u = &self.0;
        let v = &other.0;
        let mut w = [0u64; 6];

        #[inline(always)]
        fn split(x: u128) -> (u64, u64) {
            ((x & 0xffffffffffffffff) as u64, (x >> 64) as u64)
        }

        unroll! {
        for j in 0..3 {
            let mut carry = 0u64;
            unroll! {
            for i in 0..3 {
                let (lo, hi) = split(u[i] as u128 * v[j] as u128);
                let w_ipj = &mut w[i + j];
                let (tmp, o1) = w_ipj.overflowing_add(lo);
                let (tmp, o2) = tmp.overflowing_add(carry);
                *w_ipj = tmp;
                carry = hi + o1 as u64 + o2 as u64;
            }
            }
        }
        }

        Self([w[0], w[1], w[2]])
    }
}

impl ops::Mul<U256> for U256 {
    type Output = Self;

    #[inline(always)]
    #[allow(unused_assignments)]
    fn mul(self, other: Self) -> Self {
        let u = &self.0;
        let v = &other.0;
        let mut w = [0u64; 8];

        #[inline(always)]
        fn split(x: u128) -> (u64, u64) {
            ((x & 0xffffffffffffffff) as u64, (x >> 64) as u64)
        }

        unroll! {
        for j in 0..4 {
            let mut carry = 0u64;
            unroll! {
            for i in 0..4 {
                let (lo, hi) = split(u[i] as u128 * v[j] as u128);
                let w_ipj = &mut w[i + j];
                let (tmp, o1) = w_ipj.overflowing_add(lo);
                let (tmp, o2) = tmp.overflowing_add(carry);
                *w_ipj = tmp;
                carry = hi + o1 as u64 + o2 as u64;
            }
            }
        }
        }

        Self([w[0], w[1], w[2], w[3]])
    }
}

impl ops::MulAssign<U192> for U192 {
    #[inline(always)]
    fn mul_assign(&mut self, other: Self) {
        *self = *self * other;
    }
}

impl ops::MulAssign<U256> for U256 {
    #[inline(always)]
    fn mul_assign(&mut self, other: Self) {
        *self = *self * other;
    }
}

impl ops::Mul<u64> for U192 {
    type Output = Self;

    #[inline(always)]
    #[allow(unused_assignments)]
    fn mul(self, other: u64) -> Self {
        let u = &self.0;
        let mut w = [0u64; 4];

        #[inline(always)]
        fn split(x: u128) -> (u64, u64) {
            ((x & 0xffffffffffffffff) as u64, (x >> 64) as u64)
        }

        let mut carry = 0u64;
        unroll! {
        for i in 0..3 {
            let (lo, hi) = split(u[i] as u128 * other as u128);
            let w_ipj = &mut w[i];
            let (tmp, o1) = w_ipj.overflowing_add(lo);
            let (tmp, o2) = tmp.overflowing_add(carry);
            *w_ipj = tmp;
            carry = hi + o1 as u64 + o2 as u64;
        }
        }

        Self([w[0], w[1], w[2]])
    }
}

impl ops::Mul<u64> for U256 {
    type Output = Self;

    #[inline(always)]
    #[allow(unused_assignments)]
    fn mul(self, other: u64) -> Self {
        let u = &self.0;
        let mut w = [0u64; 5];

        #[inline(always)]
        fn split(x: u128) -> (u64, u64) {
            ((x & 0xffffffffffffffff) as u64, (x >> 64) as u64)
        }

        let mut carry = 0u64;
        unroll! {
        for i in 0..4 {
            let (lo, hi) = split(u[i] as u128 * other as u128);
            let w_ipj = &mut w[i];
            let (tmp, o1) = w_ipj.overflowing_add(lo);
            let (tmp, o2) = tmp.overflowing_add(carry);
            *w_ipj = tmp;
            carry = hi + o1 as u64 + o2 as u64;
        }
        }

        Self([w[0], w[1], w[2], w[3]])
    }
}

impl ops::Not for U192 {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Self {
        let u = &self.0;
        Self([!u[0], !u[1], !u[2]])
    }
}

impl ops::Not for U256 {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Self {
        let u = &self.0;
        Self([!u[0], !u[1], !u[2], !u[3]])
    }
}

impl ops::BitAnd<U192> for U192 {
    type Output = Self;

    #[inline(always)]
    fn bitand(self, other: Self) -> Self {
        let u = &self.0;
        let v = &other.0;
        Self([u[0] & v[0], u[1] & v[1], u[2] & v[2]])
    }
}

impl ops::BitAnd<U256> for U256 {
    type Output = Self;

    #[inline(always)]
    fn bitand(self, other: Self) -> Self {
        let u = &self.0;
        let v = &other.0;
        Self([u[0] & v[0], u[1] & v[1], u[2] & v[2], u[3] & v[3]])
    }
}

impl ops::BitAndAssign<U192> for U192 {
    #[inline(always)]
    fn bitand_assign(&mut self, other: Self) {
        *self = *self & other;
    }
}

impl ops::BitAndAssign<U256> for U256 {
    #[inline(always)]
    fn bitand_assign(&mut self, other: Self) {
        *self = *self & other;
    }
}

impl ops::Shl<u32> for U192 {
    type Output = Self;

    #[inline(always)]
    fn shl(self, other: u32) -> Self {
        let u = &self.0;
        if other >= 192 {
            Self::ZERO
        } else if other >= 128 {
            debug_assert!(other < 192);
            Self([0, 0, u[0].wrapping_shl(other - 128)])
        } else if other >= 64 {
            debug_assert!(other < 128);
            Self([
                0u64,
                u[0].wrapping_shl(other - 64),
                u[1].wrapping_shl(other - 64) | u[0].wrapping_shr(128 - other),
            ])
        } else {
            debug_assert!(other < 64);
            Self([
                u[0].wrapping_shl(other),
                u[1].wrapping_shl(other) | u[0].wrapping_shr(64 - other),
                u[2].wrapping_shl(other) | u[1].wrapping_shr(64 - other),
            ])
        }
    }
}
impl ops::Shl<usize> for U192 {
    type Output = Self;

    #[inline(always)]
    fn shl(self, other: usize) -> Self {
        self << other as u32
    }
}

impl ops::Shl<u32> for U256 {
    type Output = Self;

    #[inline(always)]
    fn shl(self, other: u32) -> Self {
        let u = &self.0;
        if other >= 256 {
            Self::ZERO
        } else if other >= 192 {
            debug_assert!(other < 256);
            Self([0u64, 0u64, 0u64, u[0].wrapping_shl(other - 192)])
        } else if other >= 128 {
            debug_assert!(other < 192);
            Self([
                0u64,
                0u64,
                u[0].wrapping_shl(other - 128),
                u[1].wrapping_shl(other - 128) | u[0].wrapping_shr(192 - other),
            ])
        } else if other >= 64 {
            debug_assert!(other < 128);
            Self([
                0u64,
                u[0].wrapping_shl(other - 64),
                u[1].wrapping_shl(other - 64) | u[0].wrapping_shr(128 - other),
                u[2].wrapping_shl(other - 64) | u[1].wrapping_shr(128 - other),
            ])
        } else {
            debug_assert!(other < 64);
            Self([
                u[0].wrapping_shl(other),
                u[1].wrapping_shl(other) | u[0].wrapping_shr(64 - other),
                u[2].wrapping_shl(other) | u[1].wrapping_shr(64 - other),
                u[3].wrapping_shl(other) | u[2].wrapping_shr(64 - other),
            ])
        }
    }
}
impl ops::Shl<usize> for U256 {
    type Output = Self;

    #[inline(always)]
    fn shl(self, other: usize) -> Self {
        self << other as u32
    }
}

impl ops::ShlAssign<u32> for U192 {
    #[inline(always)]
    fn shl_assign(&mut self, other: u32) {
        *self = *self << other;
    }
}
impl ops::ShlAssign<usize> for U192 {
    #[inline(always)]
    fn shl_assign(&mut self, other: usize) {
        *self <<= other as u32;
    }
}

impl ops::ShlAssign<u32> for U256 {
    #[inline(always)]
    fn shl_assign(&mut self, other: u32) {
        *self = *self << other;
    }
}
impl ops::ShlAssign<usize> for U256 {
    #[inline(always)]
    fn shl_assign(&mut self, other: usize) {
        *self <<= other as u32;
    }
}

impl fmt::Display for U192 {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:x}{:x}{:x}", self.0[2], self.0[1], self.0[0])
    }
}

impl fmt::Display for U256 {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "0x{:x}{:x}{:x}{:x}",
            self.0[3], self.0[2], self.0[1], self.0[0]
        )
    }
}

impl Distribution<U192> for Standard {
    #[inline(always)]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> U192 {
        U192(rng.gen::<[u64; 3]>())
    }
}

impl Distribution<U256> for Standard {
    #[inline(always)]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> U256 {
        U256(rng.gen::<[u64; 4]>())
    }
}

mod tests {

    use super::U192;
    use super::U256;

    #[test]
    fn u192_add() {
        let a = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let b = U192([0x9720cfadd82d0932, 0x5ec1fdbbbb0c144a, 0xddd475bf0f773b7f]);
        let c = U192([0x9a4320fd87e15697, 0x6196750047664f49, 0x1b27a155b1da87d3]);
        assert_eq!(a + b, c);
    }

    #[test]
    fn u192_sub() {
        let a = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let b = U192([0x9720cfadd82d0932, 0x5ec1fdbbbb0c144a, 0xddd475bf0f773b7f]);
        let c = U192([0x6c0181a1d7874433, 0xa4127988d14e26b4, 0x5f7eb5d792ec10d4]);
        let d = U192([0x93fe7e5e2878bbcd, 0x5bed86772eb1d94b, 0xa0814a286d13ef2b]);
        assert_eq!(a - b, c);
        assert_eq!(b - a, d);
    }

    #[test]
    fn u192_neg() {
        let a = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let b = U192([0xfcddaeb0504bb29b, 0xfd2b88bb73a5c500, 0xc2acd4695d9cb3ab]);
        assert_eq!(-a, b);
        assert_eq!(-U256::ZERO, U256::ZERO);
        assert_eq!(-U256::ONE, U256::MAX);
    }

    #[test]
    fn u192_mul() {
        let a = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let b = U192([0x9720cfadd82d0932, 0x5ec1fdbbbb0c144a, 0xddd475bf0f773b7f]);
        let c = U192([0x1fdeaafd7ab0aaba, 0x00423b9a6f9af3dd, 0x62fd92e49acb19a6]);
        assert_eq!(a * b, c);
    }

    #[test]
    fn u192_not() {
        let a = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let c = U192([0xfcddaeb0504bb29a, 0xfd2b88bb73a5c500, 0xc2acd4695d9cb3ab]);
        assert_eq!(!a, c);
    }

    #[test]
    fn u192_bit_and() {
        let a = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let b = U192([0x9720cfadd82d0932, 0x5ec1fdbbbb0c144a, 0xddd475bf0f773b7f]);
        let c = U192([0x0320410d88240920, 0x02c075008808104a, 0x1d50219602630854]);
        assert_eq!(a & b, c);
    }

    #[test]
    fn u192_shl() {
        let a = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let c = U192([0xd135940000000000, 0x68ebfc0c89453ebe, 0x8d31500b51dd1231]);
        let d = U192([0x0000000000000000, 0x22514fafb44d6500, 0xd477448c5a3aff03]);
        let e = U192([0x0000000000000000, 0x0000000000000000, 0x8a7d7da26b280000]);
        let n_c = 42usize;
        let n_d = 72usize;
        let n_e = 147usize;
        assert_eq!(a << n_c, c);
        assert_eq!(a << n_d, d);
        assert_eq!(a << n_e, e);
    }

    #[test]
    fn u256_add() {
        let a = U256([
            0x0322514fafb44d65,
            0x02d477448c5a3aff,
            0x3d532b96a2634c54,
            0xb980672899a0532f,
        ]);
        let b = U256([
            0x9720cfadd82d0932,
            0x5ec1fdbbbb0c144a,
            0xddd475bf0f773b7f,
            0xe07122b2558224a7,
        ]);
        let c = U256([
            0x9a4320fd87e15697,
            0x6196750047664f49,
            0x1b27a155b1da87d3,
            0x99f189daef2277d7,
        ]);
        assert_eq!(a + b, c);
    }

    #[test]
    fn u256_sub() {
        let a = U256([
            0x0322514fafb44d65,
            0x02d477448c5a3aff,
            0x3d532b96a2634c54,
            0xb980672899a0532f,
        ]);
        let b = U256([
            0x9720cfadd82d0932,
            0x5ec1fdbbbb0c144a,
            0xddd475bf0f773b7f,
            0xe07122b2558224a7,
        ]);
        let c = U256([
            0x6c0181a1d7874433,
            0xa4127988d14e26b4,
            0x5f7eb5d792ec10d4,
            0xd90f4476441e2e87,
        ]);
        let d = U256([
            0x93fe7e5e2878bbcd,
            0x5bed86772eb1d94b,
            0xa0814a286d13ef2b,
            0x26f0bb89bbe1d178,
        ]);
        assert_eq!(a - b, c);
        assert_eq!(b - a, d);
    }

    #[test]
    fn u256_neg() {
        let a = U256([
            0x0322514fafb44d65,
            0x02d477448c5a3aff,
            0x3d532b96a2634c54,
            0xb980672899a0532f,
        ]);
        let b = U256([
            0xfcddaeb0504bb29b,
            0xfd2b88bb73a5c500,
            0xc2acd4695d9cb3ab,
            0x467f98d7665facd0,
        ]);
        assert_eq!(-a, b);
        assert_eq!(-U256::ZERO, U256::ZERO);
        assert_eq!(-U256::ONE, U256::MAX);
    }

    #[test]
    fn u256_mul() {
        let a = U256([
            0x0322514fafb44d65,
            0x02d477448c5a3aff,
            0x3d532b96a2634c54,
            0xb980672899a0532f,
        ]);
        let b = U256([
            0x9720cfadd82d0932,
            0x5ec1fdbbbb0c144a,
            0xddd475bf0f773b7f,
            0xe07122b2558224a7,
        ]);
        let c = U256([
            0x1fdeaafd7ab0aaba,
            0x00423b9a6f9af3dd,
            0x62fd92e49acb19a6,
            0x5b9f4854a39e3316,
        ]);
        assert_eq!(a * b, c);
    }

    #[test]
    fn u256_not() {
        let a = U256([
            0x0322514fafb44d65,
            0x02d477448c5a3aff,
            0x3d532b96a2634c54,
            0xb980672899a0532f,
        ]);
        let c = U256([
            0xfcddaeb0504bb29a,
            0xfd2b88bb73a5c500,
            0xc2acd4695d9cb3ab,
            0x467f98d7665facd0,
        ]);
        assert_eq!(!a, c);
    }

    #[test]
    fn u256_bit_and() {
        let a = U256([
            0x0322514fafb44d65,
            0x02d477448c5a3aff,
            0x3d532b96a2634c54,
            0xb980672899a0532f,
        ]);
        let b = U256([
            0x9720cfadd82d0932,
            0x5ec1fdbbbb0c144a,
            0xddd475bf0f773b7f,
            0xe07122b2558224a7,
        ]);
        let c = U256([
            0x0320410d88240920,
            0x02c075008808104a,
            0x1d50219602630854,
            0xa000222011800027,
        ]);
        assert_eq!(a & b, c);
    }

    #[test]
    fn u256_shl() {
        let a = U256([
            0x0322514fafb44d65,
            0x02d477448c5a3aff,
            0x3d532b96a2634c54,
            0xb980672899a0532f,
        ]);
        let c = U256([
            0xd135940000000000,
            0x68ebfc0c89453ebe,
            0x8d31500b51dd1231,
            0x814cbcf54cae5a89,
        ]);
        let d = U256([
            0x0000000000000000,
            0x22514fafb44d6500,
            0xd477448c5a3aff03,
            0x532b96a2634c5402,
        ]);
        let e = U256([
            0x0000000000000000,
            0x0000000000000000,
            0x8a7d7da26b280000,
            0xba2462d1d7f81912,
        ]);
        let f = U256([
            0x0000000000000000,
            0x0000000000000000,
            0x0000000000000000,
            0x22514fafb44d6500,
        ]);
        let n_c = 42usize;
        let n_d = 72usize;
        let n_e = 147usize;
        let n_f = 200usize;
        assert_eq!(a << n_c, c);
        assert_eq!(a << n_d, d);
        assert_eq!(a << n_e, e);
        assert_eq!(a << n_f, f);
    }
}
