use crate::{
    channel::{AbstractChannel, Receivable, Sendable},
    ring::NewRing,
    Block, AES_HASH,
};
use primitive_types::U256;
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use std::{
    cmp::{Eq, PartialEq},
    convert::From,
    fmt, io,
    iter::Sum,
    mem,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
    slice,
};
use uint::construct_uint;

#[derive(Copy, Clone)]
#[repr(C, align(16))]
pub struct Z2r<const BIT_LENGTH: usize>(u128);

impl<const BIT_LENGTH: usize> Z2r<BIT_LENGTH> {
    pub const BYTE_LENGTH: usize = (BIT_LENGTH + 7) / 8;
    pub const BIT_MASK: u128 = (1u128 << BIT_LENGTH) - 1;
}

impl<const BIT_LENGTH: usize> From<u128> for Z2r<BIT_LENGTH> {
    #[inline(always)]
    fn from(x: u128) -> Self {
        Self(x)
    }
}

impl<const BIT_LENGTH: usize> From<Block> for Z2r<BIT_LENGTH> {
    #[inline(always)]
    fn from(x: Block) -> Self {
        Self(x.extract_u128())
    }
}

impl<const BIT_LENGTH: usize> PartialEq<Self> for Z2r<BIT_LENGTH> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.reduce().0 == other.reduce().0
    }
}
impl<const BIT_LENGTH: usize> Eq for Z2r<BIT_LENGTH> {}

impl<const BIT_LENGTH: usize> Add<Self> for Z2r<BIT_LENGTH> {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Z2r(self.0.wrapping_add(rhs.0))
    }
}

impl<const BIT_LENGTH: usize> AddAssign<Self> for Z2r<BIT_LENGTH> {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_add(rhs.0);
    }
}

impl<const BIT_LENGTH: usize> Sub<Self> for Z2r<BIT_LENGTH> {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Z2r(self.0.wrapping_sub(rhs.0))
    }
}

impl<const BIT_LENGTH: usize> SubAssign<Self> for Z2r<BIT_LENGTH> {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_sub(rhs.0);
    }
}

impl<const BIT_LENGTH: usize> Mul<Self> for Z2r<BIT_LENGTH> {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        Z2r(self.0.wrapping_mul(rhs.0))
    }
}

impl<const BIT_LENGTH: usize> MulAssign<Self> for Z2r<BIT_LENGTH> {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_mul(rhs.0);
    }
}

impl<const BIT_LENGTH: usize> Neg for Z2r<BIT_LENGTH> {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self {
        Z2r(self.0.wrapping_neg())
    }
}

impl<const BIT_LENGTH: usize> Sum for Z2r<BIT_LENGTH> {
    #[inline(always)]
    // #[inline(never)]
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut s = 0u128;
        for x in iter {
            s = s.wrapping_add(x.0);
        }
        Z2r(s)
        // Z2r(iter.map(|x| Wrapping(x.0)).sum::<Wrapping<u128>>().0)
    }
}

impl<const BIT_LENGTH: usize> NewRing for Z2r<BIT_LENGTH> {
    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);
    const BIT_LENGTH: usize = BIT_LENGTH;
    const BYTE_LENGTH: usize = Z2r::<BIT_LENGTH>::BYTE_LENGTH;

    #[inline(always)]
    fn reduce(&self) -> Self {
        Self(self.0 & Self::BIT_MASK)
    }

    #[inline(always)]
    fn is_reduced(&self) -> bool {
        self.0 & (!Self::BIT_MASK) == 0
    }

    #[inline(always)]
    fn reduce_to<const BITS: usize>(&self) -> Self {
        let mask: u128 = (1u128 << BITS) - 1;
        Self(self.0 & mask)
    }

    #[inline(always)]
    fn is_reduced_to<const BITS: usize>(&self) -> bool {
        let mask: u128 = !((1u128 << BITS) - 1);
        self.0 & mask == 0
    }

    #[inline(always)]
    fn sum(slice: &[Self]) -> Self {
        let mut s = Self::ZERO;
        for &x in slice {
            s += x;
        }
        s
    }
}

impl<const BIT_LENGTH: usize> Default for Z2r<BIT_LENGTH> {
    #[inline(always)]
    fn default() -> Self {
        Self::ZERO
    }
}

impl<const BIT_LENGTH: usize> AsRef<[u8]> for Z2r<BIT_LENGTH> {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                &*(self as *const Z2r<BIT_LENGTH> as *const u8),
                mem::size_of::<Self>(),
            )
        }
    }
}

impl<const BIT_LENGTH: usize> AsMut<[u8]> for Z2r<BIT_LENGTH> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(
                &mut *(self as *mut Z2r<BIT_LENGTH> as *mut u8),
                mem::size_of::<Self>(),
            )
        }
    }
}

impl<const BIT_LENGTH: usize> fmt::Debug for Z2r<BIT_LENGTH> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Z2r<{}>({}{})",
            BIT_LENGTH,
            self.reduce().0,
            if self.is_reduced() { "" } else { "*" }
        )
    }
}

impl<const BIT_LENGTH: usize> Distribution<Z2r<BIT_LENGTH>> for Standard {
    #[inline(always)]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Z2r<BIT_LENGTH> {
        Z2r::<BIT_LENGTH>(rng.gen())
    }
}

impl<const BIT_LENGTH: usize> Receivable for Z2r<BIT_LENGTH> {
    #[inline(always)]
    fn receive<C: AbstractChannel>(chan: &mut C) -> io::Result<Self> {
        let mut v = Self::default();
        chan.read_bytes(v.as_mut())?;
        Ok(v.reduce())
    }
}

impl<'a, const BIT_LENGTH: usize> Sendable for &Z2r<BIT_LENGTH> {
    #[inline(always)]
    fn send<C: AbstractChannel>(self, chan: &mut C) -> io::Result<()> {
        chan.write_bytes(self.reduce().as_ref())
    }
}

construct_uint! {
    /// 192-bit unsigned integer.
    #[cfg_attr(feature = "scale-info", derive(TypeInfo))]
    pub struct U192(3);
}

#[derive(Copy, Clone)]
#[repr(C, align(8))]
pub struct Z2rU192<const BIT_LENGTH: usize>(U192);

const fn compute_bit_mask_192(bit_length: usize) -> [u64; 3] {
    [
        if bit_length >= 64 {
            0xffffffffffffffffu64
        } else if bit_length == 0 {
            0
        } else {
            (1u64 << bit_length).wrapping_sub(1)
        },
        if bit_length >= 128 {
            0xffffffffffffffffu64
        } else if bit_length <= 64 {
            0
        } else {
            (1u64 << (bit_length - 64)).wrapping_sub(1)
        },
        if bit_length >= 192 {
            0xffffffffffffffffu64
        } else if bit_length <= 128 {
            0
        } else {
            (1u64 << (bit_length - 128)).wrapping_sub(1)
        },
    ]
}

impl<const BIT_LENGTH: usize> Z2rU192<BIT_LENGTH> {
    pub const BYTE_LENGTH: usize = (BIT_LENGTH + 7) / 8;
    pub const BIT_MASK: U192 = U192(compute_bit_mask_192(BIT_LENGTH));
}

impl<const BIT_LENGTH: usize> From<U192> for Z2rU192<BIT_LENGTH> {
    #[inline(always)]
    fn from(x: U192) -> Self {
        Self(x)
    }
}

impl<const BIT_LENGTH: usize> From<Block> for Z2rU192<BIT_LENGTH> {
    #[inline(always)]
    fn from(x: Block) -> Self {
        let o1: Block = AES_HASH.cr_hash(Block::default(), x).into();
        let o2: Block = (u128::from(o1).wrapping_add(u128::from(x))).into();
        let (q0, q1): (u64, u64) = o1.into();
        let (q2, _): (u64, u64) = o2.into();
        Self(U192([q0, q1, q2]))
    }
}

impl<const BIT_LENGTH: usize> PartialEq<Self> for Z2rU192<BIT_LENGTH> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.reduce().0 == other.reduce().0
    }
}
impl<const BIT_LENGTH: usize> Eq for Z2rU192<BIT_LENGTH> {}

impl<const BIT_LENGTH: usize> Add<Self> for Z2rU192<BIT_LENGTH> {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Z2rU192(self.0.overflowing_add(rhs.0).0)
    }
}

impl<const BIT_LENGTH: usize> AddAssign<Self> for Z2rU192<BIT_LENGTH> {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.overflowing_add(rhs.0).0;
    }
}

impl<const BIT_LENGTH: usize> Sub<Self> for Z2rU192<BIT_LENGTH> {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Z2rU192(self.0.overflowing_sub(rhs.0).0)
    }
}

impl<const BIT_LENGTH: usize> SubAssign<Self> for Z2rU192<BIT_LENGTH> {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.overflowing_sub(rhs.0).0;
    }
}

impl<const BIT_LENGTH: usize> Mul<Self> for Z2rU192<BIT_LENGTH> {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        Z2rU192(self.0.overflowing_mul(rhs.0).0)
    }
}

impl<const BIT_LENGTH: usize> MulAssign<Self> for Z2rU192<BIT_LENGTH> {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        self.0 = self.0.overflowing_mul(rhs.0).0;
    }
}

impl<const BIT_LENGTH: usize> Neg for Z2rU192<BIT_LENGTH> {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self {
        // U192::overflowing_neg is currently broken: https://github.com/paritytech/parity-common/pull/611
        // Z2rU192(self.0.overflowing_neg().0)
        Z2rU192(U192::zero().overflowing_sub(self.0).0)
    }
}

impl<const BIT_LENGTH: usize> Sum for Z2rU192<BIT_LENGTH> {
    #[inline(always)]
    // #[inline(never)]
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut s = U192::zero();
        for x in iter {
            s = s.overflowing_add(x.0).0;
        }
        Z2rU192(s)
    }
}

impl<const BIT_LENGTH: usize> NewRing for Z2rU192<BIT_LENGTH> {
    const ZERO: Self = Self(U192::zero());
    const ONE: Self = Self(U192([1, 0, 0]));
    const BIT_LENGTH: usize = BIT_LENGTH;
    const BYTE_LENGTH: usize = Z2r::<BIT_LENGTH>::BYTE_LENGTH;

    #[inline(always)]
    fn reduce(&self) -> Self {
        Self(self.0 & Self::BIT_MASK)
    }

    #[inline(always)]
    fn is_reduced(&self) -> bool {
        (self.0 & (!Self::BIT_MASK)).is_zero()
    }

    #[inline(always)]
    fn reduce_to<const BITS: usize>(&self) -> Self {
        let mask: U192 = (U192::one() << BITS) - 1;
        Self(self.0 & mask)
    }

    #[inline(always)]
    fn is_reduced_to<const BITS: usize>(&self) -> bool {
        let mask: U192 = !((U192::one() << BITS) - 1);
        (self.0 & mask).is_zero()
    }

    #[inline(always)]
    fn sum(slice: &[Self]) -> Self {
        let mut s = Self::ZERO;
        for &x in slice {
            s += x;
        }
        s
    }
}

impl<const BIT_LENGTH: usize> Default for Z2rU192<BIT_LENGTH> {
    #[inline(always)]
    fn default() -> Self {
        Self::ZERO
    }
}

impl<const BIT_LENGTH: usize> AsRef<[u8]> for Z2rU192<BIT_LENGTH> {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                &*(self as *const Z2rU192<BIT_LENGTH> as *const u8),
                mem::size_of::<Self>(),
            )
        }
    }
}

impl<const BIT_LENGTH: usize> AsMut<[u8]> for Z2rU192<BIT_LENGTH> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(
                &mut *(self as *mut Z2rU192<BIT_LENGTH> as *mut u8),
                mem::size_of::<Self>(),
            )
        }
    }
}

impl<const BIT_LENGTH: usize> fmt::Debug for Z2rU192<BIT_LENGTH> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Z2rU192<{}>({}{})",
            BIT_LENGTH,
            self.reduce().0,
            if self.is_reduced() { "" } else { "*" }
        )
    }
}

impl<const BIT_LENGTH: usize> Distribution<Z2rU192<BIT_LENGTH>> for Standard {
    #[inline(always)]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Z2rU192<BIT_LENGTH> {
        Z2rU192::<BIT_LENGTH>(U192(rng.gen::<[u64; 3]>()))
    }
}

impl<const BIT_LENGTH: usize> Receivable for Z2rU192<BIT_LENGTH> {
    #[inline(always)]
    fn receive<C: AbstractChannel>(chan: &mut C) -> io::Result<Self> {
        let mut v = Self::default();
        chan.read_bytes(v.as_mut())?;
        Ok(v.reduce())
    }
}

impl<'a, const BIT_LENGTH: usize> Sendable for &Z2rU192<BIT_LENGTH> {
    #[inline(always)]
    fn send<C: AbstractChannel>(self, chan: &mut C) -> io::Result<()> {
        chan.write_bytes(self.reduce().as_ref())
    }
}

#[derive(Copy, Clone)]
#[repr(C, align(32))]
pub struct Z2rU256<const BIT_LENGTH: usize>(U256);

const fn compute_bit_mask_256(bit_length: usize) -> [u64; 4] {
    [
        if bit_length >= 64 {
            0xffffffffffffffffu64
        } else if bit_length == 0 {
            0
        } else {
            (1u64 << bit_length).wrapping_sub(1)
        },
        if bit_length >= 128 {
            0xffffffffffffffffu64
        } else if bit_length <= 64 {
            0
        } else {
            (1u64 << (bit_length - 64)).wrapping_sub(1)
        },
        if bit_length >= 192 {
            0xffffffffffffffffu64
        } else if bit_length <= 128 {
            0
        } else {
            (1u64 << (bit_length - 128)).wrapping_sub(1)
        },
        if bit_length >= 256 {
            0xffffffffffffffffu64
        } else if bit_length <= 192 {
            0
        } else {
            (1u64 << (bit_length - 192)).wrapping_sub(1)
        },
    ]
}

impl<const BIT_LENGTH: usize> Z2rU256<BIT_LENGTH> {
    pub const BYTE_LENGTH: usize = (BIT_LENGTH + 7) / 8;
    pub const BIT_MASK: U256 = U256(compute_bit_mask_256(BIT_LENGTH));
}

impl<const BIT_LENGTH: usize> From<U256> for Z2rU256<BIT_LENGTH> {
    #[inline(always)]
    fn from(x: U256) -> Self {
        Self(x)
    }
}

impl<const BIT_LENGTH: usize> From<Block> for Z2rU256<BIT_LENGTH> {
    #[inline(always)]
    fn from(x: Block) -> Self {
        let o1: Block = AES_HASH.cr_hash(Block::default(), x).into();
        let o2: Block = (u128::from(o1).wrapping_add(u128::from(x))).into();
        let (q0, q1): (u64, u64) = o1.into();
        let (q2, q3): (u64, u64) = o2.into();
        Self(U256([q0, q1, q2, q3]))
    }
}

impl<const BIT_LENGTH: usize> PartialEq<Self> for Z2rU256<BIT_LENGTH> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.reduce().0 == other.reduce().0
    }
}
impl<const BIT_LENGTH: usize> Eq for Z2rU256<BIT_LENGTH> {}

impl<const BIT_LENGTH: usize> Add<Self> for Z2rU256<BIT_LENGTH> {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Z2rU256(self.0.overflowing_add(rhs.0).0)
    }
}

impl<const BIT_LENGTH: usize> AddAssign<Self> for Z2rU256<BIT_LENGTH> {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.overflowing_add(rhs.0).0;
    }
}

impl<const BIT_LENGTH: usize> Sub<Self> for Z2rU256<BIT_LENGTH> {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Z2rU256(self.0.overflowing_sub(rhs.0).0)
    }
}

impl<const BIT_LENGTH: usize> SubAssign<Self> for Z2rU256<BIT_LENGTH> {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.overflowing_sub(rhs.0).0;
    }
}

impl<const BIT_LENGTH: usize> Mul<Self> for Z2rU256<BIT_LENGTH> {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        Z2rU256(self.0.overflowing_mul(rhs.0).0)
    }
}

impl<const BIT_LENGTH: usize> MulAssign<Self> for Z2rU256<BIT_LENGTH> {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        self.0 = self.0.overflowing_mul(rhs.0).0;
    }
}

impl<const BIT_LENGTH: usize> Neg for Z2rU256<BIT_LENGTH> {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self {
        // U256::overflowing_neg is currently broken: https://github.com/paritytech/parity-common/pull/611
        // Z2rU256(self.0.overflowing_neg().0)
        Z2rU256(U256::zero().overflowing_sub(self.0).0)
    }
}

impl<const BIT_LENGTH: usize> Sum for Z2rU256<BIT_LENGTH> {
    #[inline(always)]
    // #[inline(never)]
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut s = U256::zero();
        for x in iter {
            s = s.overflowing_add(x.0).0;
        }
        Z2rU256(s)
    }
}

impl<const BIT_LENGTH: usize> NewRing for Z2rU256<BIT_LENGTH> {
    const ZERO: Self = Self(U256::zero());
    const ONE: Self = Self(U256([1, 0, 0, 0]));
    const BIT_LENGTH: usize = BIT_LENGTH;
    const BYTE_LENGTH: usize = Z2r::<BIT_LENGTH>::BYTE_LENGTH;

    #[inline(always)]
    fn reduce(&self) -> Self {
        Self(self.0 & Self::BIT_MASK)
    }

    #[inline(always)]
    fn is_reduced(&self) -> bool {
        (self.0 & (!Self::BIT_MASK)).is_zero()
    }

    #[inline(always)]
    fn reduce_to<const BITS: usize>(&self) -> Self {
        let mask: U256 = (U256::one() << BITS) - 1;
        Self(self.0 & mask)
    }

    #[inline(always)]
    fn is_reduced_to<const BITS: usize>(&self) -> bool {
        let mask: U256 = !((U256::one() << BITS) - 1);
        (self.0 & mask).is_zero()
    }

    #[inline(always)]
    fn sum(slice: &[Self]) -> Self {
        let mut s = Self::ZERO;
        for &x in slice {
            s += x;
        }
        s
    }
}

impl<const BIT_LENGTH: usize> Default for Z2rU256<BIT_LENGTH> {
    #[inline(always)]
    fn default() -> Self {
        Self::ZERO
    }
}

impl<const BIT_LENGTH: usize> AsRef<[u8]> for Z2rU256<BIT_LENGTH> {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                &*(self as *const Z2rU256<BIT_LENGTH> as *const u8),
                mem::size_of::<Self>(),
            )
        }
    }
}

impl<const BIT_LENGTH: usize> AsMut<[u8]> for Z2rU256<BIT_LENGTH> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(
                &mut *(self as *mut Z2rU256<BIT_LENGTH> as *mut u8),
                mem::size_of::<Self>(),
            )
        }
    }
}

impl<const BIT_LENGTH: usize> fmt::Debug for Z2rU256<BIT_LENGTH> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Z2rU256<{}>({}{})",
            BIT_LENGTH,
            self.reduce().0,
            if self.is_reduced() { "" } else { "*" }
        )
    }
}

impl<const BIT_LENGTH: usize> Distribution<Z2rU256<BIT_LENGTH>> for Standard {
    #[inline(always)]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Z2rU256<BIT_LENGTH> {
        Z2rU256::<BIT_LENGTH>(U256(rng.gen::<[u64; 4]>()))
    }
}

impl<const BIT_LENGTH: usize> Receivable for Z2rU256<BIT_LENGTH> {
    #[inline(always)]
    fn receive<C: AbstractChannel>(chan: &mut C) -> io::Result<Self> {
        let mut v = Self::default();
        chan.read_bytes(v.as_mut())?;
        Ok(v.reduce())
    }
}

impl<'a, const BIT_LENGTH: usize> Sendable for &Z2rU256<BIT_LENGTH> {
    #[inline(always)]
    fn send<C: AbstractChannel>(self, chan: &mut C) -> io::Result<()> {
        chan.write_bytes(self.reduce().as_ref())
    }
}

// k + s, k + log s, k + 2s, k + 2s + log s

// k = 32, s = 40
pub type R72 = Z2r<72>;
pub type R78 = Z2r<78>;
pub type R112 = Z2r<112>;
pub type R118 = Z2r<118>;

// k = 64, s = 40
pub type R104 = Z2r<104>;
pub type R110 = Z2r<110>;
pub type R144 = Z2rU192<144>;
pub type R150 = Z2rU192<150>;

// k = 32, s = 80
// pub type R112 = Z2r<112>;
pub type R119 = Z2r<119>;
pub type R196 = Z2rU256<196>;
pub type R203 = Z2rU256<203>;

// k = 64, s = 80
// pub type R144 = Z2rU256<114>;
pub type R151 = Z2rU192<151>;
pub type R224 = Z2rU256<224>;
pub type R231 = Z2rU256<231>;

#[cfg(test)]
mod tests {
    use super::U192;
    use super::{Z2rU192, Z2rU256, R104};
    use crate::{channel::AbstractChannel, ring::NewRing, unix_channel_pair, Block};
    use primitive_types::U256;
    use rand::{rngs::OsRng, Rng};
    use u256_literal::u256;

    type R144_256 = Z2rU256<144>;
    type R144_192 = Z2rU192<144>;

    const BIT_LENGTH_104: usize = 104;
    const MOD_104: u128 = 1 << BIT_LENGTH_104;

    // const BIT_LENGTH_144: usize = 144;
    const MOD_144_256: U256 = u256!(0x1000000000000000000000000000000000000);
    const MOD_144_192: U192 = U192([0x0000000000000000, 0x0000000000000000, 0x10000]);

    #[test]
    fn test_z2ru128_constants() {
        assert_eq!(R104::default(), R104::from(0u128));
        assert_eq!(R104::ZERO, R104::from(0u128));
        assert_eq!(R104::ONE, R104::from(1u128));
    }

    #[test]
    fn test_z2ru128_add() {
        let a: u128 = OsRng.gen_range(0, MOD_104);
        let b: u128 = OsRng.gen_range(0, MOD_104);
        let c = a.wrapping_add(b);
        let z_a = R104::from(a);
        let z_b = R104::from(b);
        let z_c = R104::from(c);
        assert_eq!(z_a + z_b, z_c);
    }

    #[test]
    fn test_z2ru128_add_assign() {
        let a: u128 = OsRng.gen_range(0, MOD_104);
        let b: u128 = OsRng.gen_range(0, MOD_104);
        let c = a.wrapping_add(b);
        let mut z_a = R104::from(a);
        let z_b = R104::from(b);
        z_a += z_b;
        let z_c = R104::from(c);
        assert_eq!(z_a, z_c);
    }

    #[test]
    fn test_z2ru128_sub() {
        let a: u128 = OsRng.gen_range(0, MOD_104);
        let b: u128 = OsRng.gen_range(0, MOD_104);
        let c = a.wrapping_sub(b);
        let z_a = R104::from(a);
        let z_b = R104::from(b);
        let z_c = R104::from(c);
        assert_eq!(z_a - z_b, z_c);
    }

    #[test]
    fn test_z2ru128_sub_assign() {
        let a: u128 = OsRng.gen_range(0, MOD_104);
        let b: u128 = OsRng.gen_range(0, MOD_104);
        let c = a.wrapping_sub(b);
        let mut z_a = R104::from(a);
        let z_b = R104::from(b);
        z_a -= z_b;
        let z_c = R104::from(c);
        assert_eq!(z_a, z_c);
    }

    #[test]
    fn test_z2ru128_mul() {
        let a: u128 = OsRng.gen_range(0, MOD_104);
        let b: u128 = OsRng.gen_range(0, MOD_104);
        let c = a.wrapping_mul(b);
        let z_a = R104::from(a);
        let z_b = R104::from(b);
        let z_c = R104::from(c);
        assert_eq!(z_a * z_b, z_c);
    }

    #[test]
    fn test_z2ru128_mul_assign() {
        let a: u128 = OsRng.gen_range(0, MOD_104);
        let b: u128 = OsRng.gen_range(0, MOD_104);
        let c = a.wrapping_mul(b);
        let mut z_a = R104::from(a);
        let z_b = R104::from(b);
        z_a *= z_b;
        let z_c = R104::from(c);
        assert_eq!(z_a, z_c);
    }

    #[test]
    fn test_z2ru128_neg() {
        let a: u128 = OsRng.gen_range(0, MOD_104);
        let b = a.wrapping_neg();
        let z_a = R104::from(a);
        let z_b = R104::from(b);
        assert_eq!(-z_a, z_b);
    }

    #[test]
    fn test_z2ru128_sum() {
        let bs: [u128; 32] = OsRng.gen();
        let mut z_bs = [R104::default(); 32];
        for i in 0..32 {
            z_bs[i] = R104::from(bs[i]);
        }
        let sum_bs = {
            let mut s = 0u128;
            for b in bs {
                s = s.wrapping_add(b);
            }
            s
        };
        let sum_z_bs: R104 = z_bs.iter().copied().sum();
        assert_eq!(sum_z_bs, R104::from(sum_bs));
    }

    #[test]
    fn test_z2ru128_eq() {
        let a: u128 = OsRng.gen_range(0, MOD_104);
        let b = a + MOD_104;
        let z_a = R104::from(a);
        let z_b = R104::from(b);
        assert_eq!(z_a, z_b);
    }

    #[test]
    fn test_z2ru128_reduce() {
        let a: u128 = OsRng.gen_range(0, MOD_104);
        let b = a + MOD_104;
        let z_a = R104::from(a);
        let z_b = R104::from(b);
        assert!(z_a.is_reduced());
        assert!(z_b.reduce().is_reduced());
        assert!(z_b.reduce() == z_a);
    }

    #[test]
    fn test_z2ru128_reduce_to() {
        let a: u128 = OsRng.gen_range(0, MOD_104);
        let b = a % (1 << 40);
        let z_a = R104::from(a);
        let z_b = R104::from(b);
        assert_eq!(z_a.reduce_to::<40>(), z_b);
        assert!(z_a.reduce_to::<40>().is_reduced_to::<40>());
        assert!(z_b.is_reduced_to::<40>());
    }

    #[test]
    fn test_z2ru128_from_block() {
        let b: Block = OsRng.gen();
        let z = R104::from(b);
        assert_eq!(z, R104::from(b.extract_u128()));
        assert_eq!(z.reduce_to::<64>(), R104::from(b.extract_0_u64() as u128));
    }

    #[test]
    fn test_z2ru128_as_ref() {
        let b: Block = OsRng.gen();
        let z = R104::from(b);
        let b_slice: &[u8] = b.as_ref();
        let z_slice: &[u8] = z.as_ref();
        assert_eq!(z_slice.len(), 16);
        assert_eq!(z_slice, b_slice);
    }

    #[test]
    fn test_z2ru128_as_mut() {
        let a: R104 = OsRng.gen();
        let mut b = a;
        let b_slice: &mut [u8] = b.as_mut();
        b_slice[5..16].fill(0u8);
        assert_eq!(b_slice.len(), 16);
        assert_eq!(a.reduce_to::<40>(), b);
    }

    #[test]
    fn test_z2ru128_send_receive() {
        let (mut channel_p, mut channel_v) = unix_channel_pair();
        let a: R104 = OsRng.gen();
        assert!(!a.is_reduced());
        channel_p.send(&a).unwrap();
        let b: R104 = channel_v.receive().unwrap();
        assert!(b.is_reduced());
        assert_eq!(a, b);

        let xs: [R104; 32] = OsRng.gen();
        let mut ys = [R104::default(); 32];
        channel_v.send(&xs).unwrap();
        channel_p.receive_into(&mut ys).unwrap();
        assert!(ys.iter().all(|y| y.is_reduced()));
        assert_eq!(xs, ys);
    }

    #[test]
    fn test_z2ru192_constants() {
        assert_eq!(MOD_144_192, U192::one() << 144);
        assert_eq!(R144_192::BIT_MASK.0[0], 0xffffffffffffffff);
        assert_eq!(R144_192::BIT_MASK.0[1], 0xffffffffffffffff);
        assert_eq!(R144_192::BIT_MASK.0[2], 0x000000000000ffff);
        assert_eq!(R144_192::default(), R144_192::from(U192::zero()));
        assert_eq!(R144_192::ZERO, R144_192::from(U192::zero()));
        assert_eq!(R144_192::ONE, R144_192::from(U192::one()));
    }

    #[test]
    fn test_z2ru192_add() {
        let a = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let b = U192([0x9720cfadd82d0932, 0x5ec1fdbbbb0c144a, 0xddd475bf0f773b7f]);
        let c = U192([0x9a4320fd87e15697, 0x6196750047664f49, 0x1b27a155b1da87d3]);
        assert_eq!(a.overflowing_add(b).0, c);
        let z_a = R144_192::from(a);
        let z_b = R144_192::from(b);
        let z_c = R144_192::from(c);
        assert_eq!(z_a + z_b, z_c);
    }

    #[test]
    fn test_z2ru192_add_assign() {
        let a = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let b = U192([0x9720cfadd82d0932, 0x5ec1fdbbbb0c144a, 0xddd475bf0f773b7f]);
        let c = U192([0x9a4320fd87e15697, 0x6196750047664f49, 0x1b27a155b1da87d3]);
        let mut z_a = R144_192::from(a);
        let z_b = R144_192::from(b);
        z_a += z_b;
        let z_c = R144_192::from(c);
        assert_eq!(z_a, z_c);
    }

    #[test]
    fn test_z2ru192_sub() {
        let a = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let b = U192([0x9720cfadd82d0932, 0x5ec1fdbbbb0c144a, 0xddd475bf0f773b7f]);
        let c = U192([0x6c0181a1d7874433, 0xa4127988d14e26b4, 0x5f7eb5d792ec10d4]);
        let d = U192([0x93fe7e5e2878bbcd, 0x5bed86772eb1d94b, 0xa0814a286d13ef2b]);
        assert_eq!(a.overflowing_sub(b).0, c);
        assert_eq!(b.overflowing_sub(a).0, d);
        let z_a = R144_192::from(a);
        let z_b = R144_192::from(b);
        let z_c = R144_192::from(c);
        let z_d = R144_192::from(d);
        assert_eq!(z_a - z_b, z_c);
        assert_eq!(z_b - z_a, z_d);
    }

    #[test]
    fn test_z2ru192_sub_assign() {
        let a = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let b = U192([0x9720cfadd82d0932, 0x5ec1fdbbbb0c144a, 0xddd475bf0f773b7f]);
        let c = U192([0x6c0181a1d7874433, 0xa4127988d14e26b4, 0x5f7eb5d792ec10d4]);
        let d = U192([0x93fe7e5e2878bbcd, 0x5bed86772eb1d94b, 0xa0814a286d13ef2b]);
        let z_a = R144_192::from(a);
        let z_b = R144_192::from(b);
        let mut z_x = z_a;
        z_x -= z_b;
        let mut z_y = z_b;
        z_y -= z_a;
        let z_c = R144_192::from(c);
        let z_d = R144_192::from(d);
        assert_eq!(z_x, z_c);
        assert_eq!(z_y, z_d);
    }

    #[test]
    fn test_z2ru192_mul() {
        let a = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let b = U192([0x9720cfadd82d0932, 0x5ec1fdbbbb0c144a, 0xddd475bf0f773b7f]);
        let c = U192([0x1fdeaafd7ab0aaba, 0x00423b9a6f9af3dd, 0x62fd92e49acb19a6]);
        assert_eq!(a.overflowing_mul(b).0, c);
        let z_a = R144_192::from(a);
        let z_b = R144_192::from(b);
        let z_c = R144_192::from(c);
        assert_eq!(z_a * z_b, z_c);
    }

    #[test]
    fn test_z2ru192_mul_assign() {
        let a = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let b = U192([0x9720cfadd82d0932, 0x5ec1fdbbbb0c144a, 0xddd475bf0f773b7f]);
        let c = U192([0x1fdeaafd7ab0aaba, 0x00423b9a6f9af3dd, 0x62fd92e49acb19a6]);
        let mut z_a = R144_192::from(a);
        let z_b = R144_192::from(b);
        z_a *= z_b;
        let z_c = R144_192::from(c);
        assert_eq!(z_a, z_c);
    }

    #[test]
    fn test_z2ru192_neg() {
        let a = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let b = U192([0xfcddaeb0504bb29b, 0xfd2b88bb73a5c500, 0xc2acd4695d9cb3ab]);
        assert_eq!(b, U192::zero().overflowing_sub(a).0);
        let z_a = R144_192::from(a);
        let z_b = R144_192::from(b);
        assert_eq!(1u128.overflowing_neg().0, 0u128.overflowing_sub(1u128).0);
        // broken: https://github.com/paritytech/parity-common/pull/611
        // assert_eq!(U192::one().overflowing_neg().0, U192::zero().overflowing_sub(U192::one()).0);
        assert_eq!(-z_a, z_b);
        assert_eq!(-R144_192::ZERO, R144_192::ZERO);
        assert_eq!(-R144_192::ONE, R144_192::ZERO - R144_192::ONE);
    }

    #[test]
    fn test_z2ru192_sum() {
        let mut bs = [U192::zero(); 32];
        let mut z_bs = [R144_192::default(); 32];
        for i in 0..32 {
            bs[i] = U192(OsRng.gen::<[u64; 3]>());
            z_bs[i] = R144_192::from(bs[i]);
        }
        let sum_bs = {
            let mut s = U192::zero();
            for b in bs {
                s = s.overflowing_add(b).0;
            }
            s
        };
        let sum_z_bs: R144_192 = z_bs.iter().copied().sum();
        assert_eq!(sum_z_bs, R144_192::from(sum_bs));
    }

    #[test]
    fn test_z2ru192_eq() {
        let a = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let b = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x0000000000004c54]);
        let c = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x0000000000014c54]);
        let d = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0xffffffffffff4c54]);
        let x = U192([0x0322514fafb44d66, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let y = U192([0x0322514fafb44d64, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let z_a = R144_192::from(a);
        let z_b = R144_192::from(b);
        let z_c = R144_192::from(c);
        let z_d = R144_192::from(d);
        let z_x = R144_192::from(x);
        let z_y = R144_192::from(y);
        assert_eq!(z_a, z_b);
        assert_eq!(z_a, z_c);
        assert_eq!(z_a, z_d);
        assert_ne!(z_a, z_x);
        assert_ne!(z_a, z_y);
    }

    #[test]
    fn test_z2ru192_reduce() {
        let a = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x3d532b96a2634c54]);
        let b = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x0000000000004c54]);
        let c = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0x0000000000014c54]);
        let d = U192([0x0322514fafb44d65, 0x02d477448c5a3aff, 0xffffffffffff4c54]);
        assert!(b < MOD_144_192);
        let z_a = R144_192::from(a);
        let z_b = R144_192::from(b);
        let z_c = R144_192::from(c);
        let z_d = R144_192::from(d);
        assert!(!z_a.is_reduced());
        assert!(z_b.is_reduced());
        assert!(!z_c.is_reduced());
        assert!(!z_d.is_reduced());
        assert!(z_a.reduce().is_reduced());
        assert!(z_b.reduce().is_reduced());
        assert!(z_c.reduce().is_reduced());
        assert!(z_d.reduce().is_reduced());
    }

    #[test]
    fn test_z2ru192_reduce_to() {
        let a: U192 = U192(OsRng.gen::<[u64; 3]>());
        let b = a % (1u128 << 80);
        let z_a = R144_192::from(a);
        let z_b = R144_192::from(b);
        assert_eq!(z_a.reduce_to::<80>(), z_b);
        assert!(z_a.reduce_to::<80>().is_reduced_to::<80>());
        assert!(z_b.is_reduced_to::<80>());
    }

    #[test]
    fn test_z2ru192_as_ref() {
        let a: U192 = U192(OsRng.gen::<[u64; 3]>());
        let z = R144_192::from(a);
        let z_slice: &[u8] = z.as_ref();
        assert_eq!(z_slice.len(), 24);
        assert_eq!(z_slice[..8], a.0[0].to_le_bytes());
        assert_eq!(z_slice[8..16], a.0[1].to_le_bytes());
        assert_eq!(z_slice[16..24], a.0[2].to_le_bytes());
    }

    #[test]
    fn test_z2ru192_as_mut() {
        let a: R144_192 = OsRng.gen();
        let mut b = a;
        let b_slice: &mut [u8] = b.as_mut();
        b_slice[10..24].fill(0u8);
        assert_eq!(b_slice.len(), 24);
        assert_eq!(a.reduce_to::<80>(), b);
    }

    #[test]
    fn test_z2ru192_send_receive() {
        let (mut channel_p, mut channel_v) = unix_channel_pair();
        let a: R144_192 = OsRng.gen();
        assert!(!a.is_reduced());
        channel_p.send(&a).unwrap();
        let b: R144_192 = channel_v.receive().unwrap();
        assert!(b.is_reduced());
        assert_eq!(a, b);

        let xs: [R144_192; 32] = OsRng.gen();
        let mut ys = [R144_192::default(); 32];
        channel_v.send(&xs).unwrap();
        channel_p.receive_into(&mut ys).unwrap();
        assert!(ys.iter().all(|y| y.is_reduced()));
        assert_eq!(xs, ys);
    }

    #[test]
    fn test_z2ru256_constants() {
        assert_eq!(MOD_144_256, U256::one() << 144);
        assert_eq!(R144_256::BIT_MASK.0[0], 0xffffffffffffffff);
        assert_eq!(R144_256::BIT_MASK.0[1], 0xffffffffffffffff);
        assert_eq!(R144_256::BIT_MASK.0[2], 0x000000000000ffff);
        assert_eq!(R144_256::BIT_MASK.0[3], 0x0000000000000000);
        assert_eq!(
            R144_256::BIT_MASK,
            u256!(0xffffffffffffffffffffffffffffffffffff)
        );
        assert_eq!(R144_256::default(), R144_256::from(u256!(0)));
        assert_eq!(R144_256::ZERO, R144_256::from(u256!(0)));
        assert_eq!(R144_256::ONE, R144_256::from(u256!(1)));
        assert_eq!(R144_256::ZERO, R144_256::from(U256::zero()));
        assert_eq!(R144_256::ONE, R144_256::from(U256::one()));
    }

    #[test]
    fn test_z2ru256_add() {
        let a = u256!(0xb980672899a0532f3d532b96a2634c5402d477448c5a3aff0322514fafb44d65);
        let b = u256!(0xe07122b2558224a7ddd475bf0f773b7f5ec1fdbbbb0c144a9720cfadd82d0932);
        let c = u256!(0x99f189daef2277d71b27a155b1da87d36196750047664f499a4320fd87e15697);
        assert_eq!(a.overflowing_add(b).0, c);
        let z_a = R144_256::from(a);
        let z_b = R144_256::from(b);
        let z_c = R144_256::from(c);
        assert_eq!(z_a + z_b, z_c);
    }

    #[test]
    fn test_z2ru256_add_assign() {
        let a = u256!(0xb980672899a0532f3d532b96a2634c5402d477448c5a3aff0322514fafb44d65);
        let b = u256!(0xe07122b2558224a7ddd475bf0f773b7f5ec1fdbbbb0c144a9720cfadd82d0932);
        let c = u256!(0x99f189daef2277d71b27a155b1da87d36196750047664f499a4320fd87e15697);
        let mut z_a = R144_256::from(a);
        let z_b = R144_256::from(b);
        z_a += z_b;
        let z_c = R144_256::from(c);
        assert_eq!(z_a, z_c);
    }

    #[test]
    fn test_z2ru256_sub() {
        let a = u256!(0xb980672899a0532f3d532b96a2634c5402d477448c5a3aff0322514fafb44d65);
        let b = u256!(0xe07122b2558224a7ddd475bf0f773b7f5ec1fdbbbb0c144a9720cfadd82d0932);
        let c = u256!(0xd90f4476441e2e875f7eb5d792ec10d4a4127988d14e26b46c0181a1d7874433);
        let d = u256!(0x26f0bb89bbe1d178a0814a286d13ef2b5bed86772eb1d94b93fe7e5e2878bbcd);
        assert_eq!(a.overflowing_sub(b).0, c);
        assert_eq!(b.overflowing_sub(a).0, d);
        let z_a = R144_256::from(a);
        let z_b = R144_256::from(b);
        let z_c = R144_256::from(c);
        let z_d = R144_256::from(d);
        assert_eq!(z_a - z_b, z_c);
        assert_eq!(z_b - z_a, z_d);
    }

    #[test]
    fn test_z2ru256_sub_assign() {
        let a = u256!(0xb980672899a0532f3d532b96a2634c5402d477448c5a3aff0322514fafb44d65);
        let b = u256!(0xe07122b2558224a7ddd475bf0f773b7f5ec1fdbbbb0c144a9720cfadd82d0932);
        let c = u256!(0xd90f4476441e2e875f7eb5d792ec10d4a4127988d14e26b46c0181a1d7874433);
        let d = u256!(0x26f0bb89bbe1d178a0814a286d13ef2b5bed86772eb1d94b93fe7e5e2878bbcd);
        let z_a = R144_256::from(a);
        let z_b = R144_256::from(b);
        let mut z_x = z_a;
        z_x -= z_b;
        let mut z_y = z_b;
        z_y -= z_a;
        let z_c = R144_256::from(c);
        let z_d = R144_256::from(d);
        assert_eq!(z_x, z_c);
        assert_eq!(z_y, z_d);
    }

    #[test]
    fn test_z2ru256_mul() {
        let a = u256!(0xb980672899a0532f3d532b96a2634c5402d477448c5a3aff0322514fafb44d65);
        let b = u256!(0xe07122b2558224a7ddd475bf0f773b7f5ec1fdbbbb0c144a9720cfadd82d0932);
        let c = u256!(0x5b9f4854a39e331662fd92e49acb19a600423b9a6f9af3dd1fdeaafd7ab0aaba);
        assert_eq!(a.overflowing_mul(b).0, c);
        let z_a = R144_256::from(a);
        let z_b = R144_256::from(b);
        let z_c = R144_256::from(c);
        assert_eq!(z_a * z_b, z_c);
    }

    #[test]
    fn test_z2ru256_mul_assign() {
        let a = u256!(0xb980672899a0532f3d532b96a2634c5402d477448c5a3aff0322514fafb44d65);
        let b = u256!(0xe07122b2558224a7ddd475bf0f773b7f5ec1fdbbbb0c144a9720cfadd82d0932);
        let c = u256!(0x5b9f4854a39e331662fd92e49acb19a600423b9a6f9af3dd1fdeaafd7ab0aaba);
        let mut z_a = R144_256::from(a);
        let z_b = R144_256::from(b);
        z_a *= z_b;
        let z_c = R144_256::from(c);
        assert_eq!(z_a, z_c);
    }

    #[test]
    fn test_z2ru256_neg() {
        let a = u256!(0xb980672899a0532f3d532b96a2634c5402d477448c5a3aff0322514fafb44d65);
        let b = u256!(0x467f98d7665facd0c2acd4695d9cb3abfd2b88bb73a5c500fcddaeb0504bb29b);
        assert_eq!(b, U256::zero().overflowing_sub(a).0);
        let z_a = R144_256::from(a);
        let z_b = R144_256::from(b);
        assert_eq!(1u128.overflowing_neg().0, 0u128.overflowing_sub(1u128).0);
        // broken: https://github.com/paritytech/parity-common/pull/611
        // assert_eq!(U256::one().overflowing_neg().0, U256::zero().overflowing_sub(U256::one()).0);
        assert_eq!(-z_a, z_b);
        assert_eq!(-R144_256::ZERO, R144_256::ZERO);
        assert_eq!(-R144_256::ONE, R144_256::ZERO - R144_256::ONE);
    }

    #[test]
    fn test_z2ru256_sum() {
        let mut bs = [U256::zero(); 32];
        let mut z_bs = [R144_256::default(); 32];
        for i in 0..32 {
            bs[i] = U256(OsRng.gen::<[u64; 4]>());
            z_bs[i] = R144_256::from(bs[i]);
        }
        let sum_bs = {
            let mut s = U256::zero();
            for b in bs {
                s = s.overflowing_add(b).0;
            }
            s
        };
        let sum_z_bs: R144_256 = z_bs.iter().copied().sum();
        assert_eq!(sum_z_bs, R144_256::from(sum_bs));
    }

    #[test]
    fn test_z2ru256_eq() {
        let a = u256!(0xb980672899a0532f3d532b96a2634c5402d477448c5a3aff0322514fafb44d65);
        let b = u256!(0x00000000000000000000000000004c5402d477448c5a3aff0322514fafb44d65);
        let c = u256!(0x00000000000000000000000000014c5402d477448c5a3aff0322514fafb44d65);
        let d = u256!(0xffffffffffffffffffffffffffff4c5402d477448c5a3aff0322514fafb44d65);
        let x = u256!(0xb980672899a0532f3d532b96a2634c5402d477448c5a3aff0322514fafb44d66);
        let y = u256!(0xb980672899a0532f3d532b96a2634c5402d477448c5a3aff0322514fafb44d64);
        let z_a = R144_256::from(a);
        let z_b = R144_256::from(b);
        let z_c = R144_256::from(c);
        let z_d = R144_256::from(d);
        let z_x = R144_256::from(x);
        let z_y = R144_256::from(y);
        assert_eq!(z_a, z_b);
        assert_eq!(z_a, z_c);
        assert_eq!(z_a, z_d);
        assert_ne!(z_a, z_x);
        assert_ne!(z_a, z_y);
    }

    #[test]
    fn test_z2ru256_reduce() {
        let a = u256!(0xb980672899a0532f3d532b96a2634c5402d477448c5a3aff0322514fafb44d65);
        let b = u256!(0x00000000000000000000000000004c5402d477448c5a3aff0322514fafb44d65);
        let c = u256!(0x00000000000000000000000000014c5402d477448c5a3aff0322514fafb44d65);
        let d = u256!(0xffffffffffffffffffffffffffff4c5402d477448c5a3aff0322514fafb44d65);
        assert!(b < MOD_144_256);
        let z_a = R144_256::from(a);
        let z_b = R144_256::from(b);
        let z_c = R144_256::from(c);
        let z_d = R144_256::from(d);
        assert!(!z_a.is_reduced());
        assert!(z_b.is_reduced());
        assert!(!z_c.is_reduced());
        assert!(!z_d.is_reduced());
        assert!(z_a.reduce().is_reduced());
        assert!(z_b.reduce().is_reduced());
        assert!(z_c.reduce().is_reduced());
        assert!(z_d.reduce().is_reduced());
    }

    #[test]
    fn test_z2ru256_reduce_to() {
        let a: U256 = U256(OsRng.gen::<[u64; 4]>());
        let b = a % (1u128 << 80);
        let z_a = R144_256::from(a);
        let z_b = R144_256::from(b);
        assert_eq!(z_a.reduce_to::<80>(), z_b);
        assert!(z_a.reduce_to::<80>().is_reduced_to::<80>());
        assert!(z_b.is_reduced_to::<80>());
    }

    #[test]
    fn test_z2ru256_as_ref() {
        let a: U256 = U256(OsRng.gen::<[u64; 4]>());
        let z = R144_256::from(a);
        let z_slice: &[u8] = z.as_ref();
        assert_eq!(z_slice.len(), 32);
        assert_eq!(z_slice[..8], a.0[0].to_le_bytes());
        assert_eq!(z_slice[8..16], a.0[1].to_le_bytes());
        assert_eq!(z_slice[16..24], a.0[2].to_le_bytes());
        assert_eq!(z_slice[24..], a.0[3].to_le_bytes());
    }

    #[test]
    fn test_z2ru256_as_mut() {
        let a: R144_256 = OsRng.gen();
        let mut b = a;
        let b_slice: &mut [u8] = b.as_mut();
        b_slice[10..32].fill(0u8);
        assert_eq!(b_slice.len(), 32);
        assert_eq!(a.reduce_to::<80>(), b);
    }

    #[test]
    fn test_z2ru256_send_receive() {
        let (mut channel_p, mut channel_v) = unix_channel_pair();
        let a: R144_256 = OsRng.gen();
        assert!(!a.is_reduced());
        channel_p.send(&a).unwrap();
        let b: R144_256 = channel_v.receive().unwrap();
        assert!(b.is_reduced());
        assert_eq!(a, b);

        let xs: [R144_256; 32] = OsRng.gen();
        let mut ys = [R144_256::default(); 32];
        channel_v.send(&xs).unwrap();
        channel_p.receive_into(&mut ys).unwrap();
        assert!(ys.iter().all(|y| y.is_reduced()));
        assert_eq!(xs, ys);
    }
}
