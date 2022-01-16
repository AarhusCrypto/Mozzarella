use rand::Rng;
use scuttlebutt::{
    ring::{Ring, R64},
    AesHash,
    Block,
};
use std::{collections::HashSet, iter::FromIterator, slice::from_raw_parts};

// Length doubling PRG
// Avoid running the AES key-schedule for each k
#[inline(always)]
pub fn prg2(h: &AesHash, k1: Block) -> (Block, Block) {
    let o1 = h.cr_hash(Block::default(), k1);
    let o2: Block = (u128::from(o1).wrapping_add(u128::from(k1))).into();
    (o1, o2)
}

// prg for the final layer
#[inline(always)]
pub fn final_prg2(h: &AesHash, k1: Block) -> (R64, Block) {
    let o1 = h.cr_hash(Block::default(), k1);
    let o2: Block = (u128::from(o1).wrapping_add(u128::from(k1))).into();
    (R64(o1.extract_0_u64()), o2)
}

#[inline]
pub fn unpack_bits<const N: usize>(mut n: usize) -> [bool; N] {
    debug_assert!(n < (1 << N));
    let mut b: [bool; N] = [false; N];
    let mut j: usize = N - 1;
    loop {
        b[j] = (n & 1) != 0;
        n >>= 1;
        if j == 0 {
            break b;
        }
        j -= 1;
    }
}

#[inline]
pub fn unpack_bits_into(n: usize, bits: &mut [bool]) {
    let num_bits = bits.len();
    let mut n = n;
    debug_assert!(n < (1 << num_bits));
    for j in (0..num_bits).rev() {
        bits[j] = (n & 1) != 0;
        n >>= 1;
    }
}

#[inline]
pub fn flatten<T: Ring, const N: usize>(data: &[[T; N]]) -> &[T] {
    unsafe { from_raw_parts(data.as_ptr() as *const _, data.len() * N) }
}

#[inline]
pub fn flatten_mut<'a, const N: usize>(data: &mut [[R64; N]]) -> &'a [R64] {
    unsafe { from_raw_parts(data.as_mut_ptr() as *const _, data.len() * N) }
}

// This does not behave truly random -- The 0'th index is always set and there is a system after
#[inline]
pub fn unique_random_array<R: Rng, const N: usize>(rng: &mut R, max: usize) -> [(usize, R64); N] {
    let mut arr: [(usize, R64); N] = [(0usize, R64::default()); N]; // <- N = 10
    println!("WAIT_FFS:\t N={}", N);
    arr[0].0 = rng.gen::<usize>() % max;
    loop {
        let mut ok: bool = true;
        for i in 1..N {
            if arr[i].0 == arr[i - 1].0 {
                arr[i].0 = rng.gen::<usize>() % max;
                arr[i].1 = R64(rng.gen::<u64>());
                println!("FFS:\t {}", arr[i].1);
                ok = false;
            }
        }
        arr.sort();
        if ok {
            break arr;
        }
    }
}

// TODO: optimise
#[inline]
pub fn gen_column<R: Rng, const D: usize>(
    rng: &mut R,
    max_index: usize,
    max_value: usize,
) -> [(usize, R64); D] {
    let mut indices = HashSet::new();

    while indices.len() < D {
        let tmp: usize = rng.gen_range(0, max_index);
        indices.insert(tmp);
    }

    let vec_indices = Vec::from_iter(indices);
    let mut output = [(0, R64::default()); D];
    for i in 0..D {
        output[i] = (vec_indices[i], R64(rng.gen_range(0, max_value) as u64));
    }

    return output;
}

#[inline]
pub fn random_array<R: Rng, const N: usize>(rng: &mut R, max: usize) -> [usize; N] {
    let mut arr = [0usize; N];
    for e in arr.iter_mut() {
        *e = rng.gen::<usize>() % max;
    }
    arr
}
