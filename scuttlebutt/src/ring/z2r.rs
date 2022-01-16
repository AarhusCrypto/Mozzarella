use crate::{
    channel::{AbstractChannel, Receivable, Sendable},
    ring::NewRing,
    Block,
    AES_HASH,
};
use primitive_types::U256;
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use std::{
    cmp::{Eq, PartialEq},
    convert::From,
    fmt,
    io,
    iter::Sum,
    mem,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
    slice,
};

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

#[derive(Copy, Clone)]
#[repr(C, align(32))]
pub struct Z2rU256<const BIT_LENGTH: usize>(U256);

const fn compute_bit_mask(bit_length: usize) -> [u64; 4] {
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
    pub const BIT_MASK: U256 = U256(compute_bit_mask(BIT_LENGTH));
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
        Z2rU256(self.0.overflowing_neg().0)
    }
}

impl<const BIT_LENGTH: usize> Sum for Z2rU256<BIT_LENGTH> {
    #[inline(always)]
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
pub type R144 = Z2rU256<144>;
pub type R150 = Z2rU256<150>;

// k = 32, s = 80
// pub type R112 = Z2r<112>;
pub type R119 = Z2r<119>;
pub type R196 = Z2rU256<196>;
pub type R203 = Z2rU256<203>;

// k = 64, s = 80
// pub type R144 = Z2rU256<114>;
pub type R151 = Z2rU256<151>;
pub type R224 = Z2rU256<224>;
pub type R231 = Z2rU256<231>;

#[cfg(test)]
mod tests {
    use super::{R104, R144};
    use crate::{channel::AbstractChannel, ring::NewRing, unix_channel_pair, Block};
    use primitive_types::U256;
    use rand::{rngs::OsRng, Rng};
    use u256_literal::u256;

    const BIT_LENGTH_104: usize = 104;
    const MOD_104: u128 = 1 << BIT_LENGTH_104;

    // const BIT_LENGTH_144: usize = 144;
    const MOD_144: U256 = u256!(0x1000000000000000000000000000000000000);

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
    fn test_z2ru256_constants() {
        assert_eq!(MOD_144, U256::one() << 144);
        eprintln!("{:?}", R144::BIT_MASK.0);
        eprintln!("a: {:x?}", 1u64);
        eprintln!("b: {:x?}", 1u64.wrapping_shl(144));
        eprintln!("c: {:x?}", 1u64.wrapping_shl(144).wrapping_sub(1));
        assert_eq!(R144::BIT_MASK.0[0], 0xffffffffffffffff);
        assert_eq!(R144::BIT_MASK.0[1], 0xffffffffffffffff);
        assert_eq!(R144::BIT_MASK.0[2], 0x000000000000ffff);
        assert_eq!(R144::BIT_MASK.0[3], 0x0000000000000000);
        assert_eq!(
            R144::BIT_MASK,
            u256!(0xffffffffffffffffffffffffffffffffffff)
        );
        assert_eq!(R144::default(), R144::from(u256!(0)));
        assert_eq!(R144::ZERO, R144::from(u256!(0)));
        assert_eq!(R144::ONE, R144::from(u256!(1)));
    }

    #[test]
    fn test_z2ru256_add() {
        let a: U256 = U256(OsRng.gen::<[u64; 4]>());
        let b: U256 = U256(OsRng.gen::<[u64; 4]>());
        let c = a.overflowing_add(b).0;
        let z_a = R144::from(a);
        let z_b = R144::from(b);
        let z_c = R144::from(c);
        assert_eq!(z_a + z_b, z_c);
    }

    #[test]
    fn test_z2ru256_add_assign() {
        let a: U256 = U256(OsRng.gen::<[u64; 4]>());
        let b: U256 = U256(OsRng.gen::<[u64; 4]>());
        let c = a.overflowing_add(b).0;
        let mut z_a = R144::from(a);
        let z_b = R144::from(b);
        z_a += z_b;
        let z_c = R144::from(c);
        assert_eq!(z_a, z_c);
    }

    #[test]
    fn test_z2ru256_sub() {
        let a: U256 = U256(OsRng.gen::<[u64; 4]>());
        let b: U256 = U256(OsRng.gen::<[u64; 4]>());
        let c = a.overflowing_sub(b).0;
        let z_a = R144::from(a);
        let z_b = R144::from(b);
        let z_c = R144::from(c);
        assert_eq!(z_a - z_b, z_c);
    }

    #[test]
    fn test_z2ru256_sub_assign() {
        let a: U256 = U256(OsRng.gen::<[u64; 4]>());
        let b: U256 = U256(OsRng.gen::<[u64; 4]>());
        let c = a.overflowing_sub(b).0;
        let mut z_a = R144::from(a);
        let z_b = R144::from(b);
        z_a -= z_b;
        let z_c = R144::from(c);
        assert_eq!(z_a, z_c);
    }

    #[test]
    fn test_z2ru256_mul() {
        let a: U256 = U256(OsRng.gen::<[u64; 4]>());
        let b: U256 = U256(OsRng.gen::<[u64; 4]>());
        let c = a.overflowing_mul(b).0;
        let z_a = R144::from(a);
        let z_b = R144::from(b);
        let z_c = R144::from(c);
        assert_eq!(z_a * z_b, z_c);
    }

    #[test]
    fn test_z2ru256_mul_assign() {
        let a: U256 = U256(OsRng.gen::<[u64; 4]>());
        let b: U256 = U256(OsRng.gen::<[u64; 4]>());
        let c = a.overflowing_mul(b).0;
        let mut z_a = R144::from(a);
        let z_b = R144::from(b);
        z_a *= z_b;
        let z_c = R144::from(c);
        assert_eq!(z_a, z_c);
    }

    #[test]
    fn test_z2ru256_neg() {
        let a: U256 = U256(OsRng.gen::<[u64; 4]>());
        let b = a.overflowing_neg().0;
        let z_a = R144::from(a);
        let z_b = R144::from(b);
        assert_eq!(-z_a, z_b);
    }

    #[test]
    fn test_z2ru256_sum() {
        let mut bs = [U256::zero(); 32];
        let mut z_bs = [R144::default(); 32];
        for i in 0..32 {
            bs[i] = U256(OsRng.gen::<[u64; 4]>());
            z_bs[i] = R144::from(bs[i]);
        }
        let sum_bs = {
            let mut s = U256::zero();
            for b in bs {
                s = s.overflowing_add(b).0;
            }
            s
        };
        let sum_z_bs: R144 = z_bs.iter().copied().sum();
        assert_eq!(sum_z_bs, R144::from(sum_bs));
    }

    #[test]
    fn test_z2ru256_eq() {
        let a: U256 = U256(OsRng.gen::<[u64; 4]>());
        let b = a + MOD_144;
        let z_a = R144::from(a);
        let z_b = R144::from(b);
        assert_eq!(z_a, z_b);
    }

    #[test]
    fn test_z2ru256_reduce() {
        // let a: U256 = U256(OsRng.gen::<[u64; 4]>()) % (U256::one() << BIT_LENGTH_144);
        let a: U256 = U256(OsRng.gen::<[u64; 4]>()) % MOD_144;
        assert!(a < MOD_144);
        let b = a + MOD_144;
        let z_a = R144::from(a);
        let z_b = R144::from(b);
        assert!(z_a.is_reduced());
        assert!(z_b.reduce().is_reduced());
        assert!(z_b.reduce() == z_a);
    }

    #[test]
    fn test_z2ru256_reduce_to() {
        let a: U256 = U256(OsRng.gen::<[u64; 4]>());
        let b = a % (1u128 << 80);
        let z_a = R144::from(a);
        let z_b = R144::from(b);
        assert_eq!(z_a.reduce_to::<80>(), z_b);
        assert!(z_a.reduce_to::<80>().is_reduced_to::<80>());
        assert!(z_b.is_reduced_to::<80>());
    }

    #[test]
    fn test_z2ru256_as_ref() {
        let a: U256 = U256(OsRng.gen::<[u64; 4]>());
        let z = R144::from(a);
        let z_slice: &[u8] = z.as_ref();
        assert_eq!(z_slice.len(), 32);
        assert_eq!(z_slice[..8], a.0[0].to_le_bytes());
        assert_eq!(z_slice[8..16], a.0[1].to_le_bytes());
        assert_eq!(z_slice[16..24], a.0[2].to_le_bytes());
        assert_eq!(z_slice[24..], a.0[3].to_le_bytes());
    }

    #[test]
    fn test_z2ru256_as_mut() {
        let a: R144 = OsRng.gen();
        let mut b = a;
        let b_slice: &mut [u8] = b.as_mut();
        b_slice[10..32].fill(0u8);
        assert_eq!(b_slice.len(), 32);
        assert_eq!(a.reduce_to::<80>(), b);
    }

    #[test]
    fn test_z2ru256_send_receive() {
        let (mut channel_p, mut channel_v) = unix_channel_pair();
        let a: R144 = OsRng.gen();
        assert!(!a.is_reduced());
        channel_p.send(&a).unwrap();
        let b: R144 = channel_v.receive().unwrap();
        assert!(b.is_reduced());
        assert_eq!(a, b);

        let xs: [R144; 32] = OsRng.gen();
        let mut ys = [R144::default(); 32];
        channel_v.send(&xs).unwrap();
        channel_p.receive_into(&mut ys).unwrap();
        assert!(ys.iter().all(|y| y.is_reduced()));
        assert_eq!(xs, ys);
    }
}