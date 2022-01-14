use crate::ot::mozzarella::cache::{prover::CachedProver, verifier::CachedVerifier};
use rand::{
    distributions::{Distribution, Standard},
    CryptoRng,
    Rng,
};
use scuttlebutt::ring::NewRing;

pub struct GenCache {}

impl GenCache {
    pub fn new_with_size<RingT: NewRing, R: CryptoRng + Rng>(
        mut rng: R,
        delta: RingT,
        size: usize,
    ) -> (CachedProver<RingT>, CachedVerifier<RingT>)
    where
        Standard: Distribution<RingT>,
    {
        let mut prover_cache_u: Vec<RingT> = Vec::with_capacity(size);
        let mut prover_cache_w: Vec<RingT> = Vec::with_capacity(size);
        let mut verifier_cache: Vec<RingT> = Vec::with_capacity(size);
        for _ in 0..size {
            let a1 = rng.gen();
            let b1 = rng.gen();
            let c1 = a1 * delta + b1;
            prover_cache_u.push(a1);
            prover_cache_w.push(c1);
            verifier_cache.push(b1);
        }
        (
            CachedProver::init(prover_cache_u, prover_cache_w),
            CachedVerifier::<RingT>::init(verifier_cache),
        )
    }

    pub fn new<RingT: NewRing, R: CryptoRng + Rng, const K: usize, const T: usize>(
        mut rng: R,
        delta: RingT,
    ) -> (CachedProver<RingT>, CachedVerifier<RingT>)
    where
        Standard: Distribution<RingT>,
    {
        // only produce K currently
        let mut prover_cache_u: Vec<RingT> = Vec::with_capacity(K + (2 * T));
        let mut prover_cache_w: Vec<RingT> = Vec::with_capacity(K + (2 * T));
        let mut verifier_cache: Vec<RingT> = Vec::with_capacity(K + (2 * T));
        for _ in 0..K {
            let a1 = rng.gen();
            let b1 = rng.gen();
            let mut tmp = a1;
            tmp *= delta;
            let mut c1 = tmp;
            c1 += b1;
            prover_cache_u.push(a1);
            prover_cache_w.push(c1);

            verifier_cache.push(b1);
        }

        // generate base voles for spsvole
        for _ in 0..T {
            let a1 = rng.gen();
            let b1 = rng.gen();
            let mut tmp = a1;
            tmp *= delta;
            let mut c1 = tmp;
            c1 += b1;

            let a2 = rng.gen();
            let b2 = rng.gen();
            let mut tmp = a2;
            tmp *= delta;
            let mut c2 = tmp;
            c2 += b2;

            verifier_cache.push(b1);
            verifier_cache.push(b2);
            prover_cache_u.push(a1);
            prover_cache_w.push(c1);
            prover_cache_u.push(a2);
            prover_cache_w.push(c2);
        }

        (
            CachedProver::init(prover_cache_u, prover_cache_w),
            CachedVerifier::init(verifier_cache),
        )
    }
}
