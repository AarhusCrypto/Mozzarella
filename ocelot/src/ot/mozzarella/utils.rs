
use scuttlebutt::{AesHash, Block};


// Length doubling PRG
// Avoid running the AES key-schedule for each k
#[inline(always)]
pub fn prg2(h: &AesHash, k1: Block) -> (Block, Block) {
    let o1 = h.cr_hash(Block::default(), k1);
    let o2: Block = (u128::from(o1).wrapping_add(u128::from(k1))).into();
    // let o2 = h.cr_hash(Block::default(), k2);
    (o1, o2)
}