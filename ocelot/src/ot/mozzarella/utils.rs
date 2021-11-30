use scuttlebutt::{AesHash, Block};
use std::slice::from_raw_parts;
use rand::Rng;
use scuttlebutt::ring::{R64, Ring};


// TODO: Create one of these that work over R64 rather than Blocks
// Length doubling PRG
// Avoid running the AES key-schedule for each k
#[inline(always)]
pub fn prg2(h: &AesHash, k1: Block) -> (Block, Block) {
    let o1 = h.cr_hash(Block::default(), k1);
    let o2: Block = (u128::from(o1).wrapping_add(u128::from(k1))).into();
    // let o2 = h.cr_hash(Block::default(), k2);
    (o1, o2)
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
pub fn flatten<T: Ring, const N: usize>(data: &[[T;N]]) -> &[T] {
    unsafe {
        from_raw_parts(data.as_ptr() as *const _, data.len() * N)
    }
}

// This does not behave truly random -- The 0'th index is always set and there is a system after
#[inline]
pub fn unique_random_array<R: Rng, const N: usize>(rng: &mut R, max: usize) -> [(usize, R64); N] {
    let mut arr:[(usize, R64); N] = [(0usize,R64::default()) ; N]; // <- N = 10
    arr[0].0 = rng.gen::<usize>() % max;
    loop {
        let mut ok: bool = true;
        for i in 1..N {
            if arr[i].0 == arr[i - 1].0 {
                arr[i].0 = rng.gen::<usize>() % max;
                arr[i].1 = R64(rng.gen::<u64>());
                ok = false;
            }
        }
        arr.sort();
        if ok {
            break arr;
        }
    }
}
