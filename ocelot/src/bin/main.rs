use std::os::unix::thread;
use std::thread::{spawn, JoinHandle};

use rand::{rngs::OsRng, Rng, RngCore, SeedableRng, CryptoRng};
use rand::distributions::{Distribution, Standard};
use ocelot::Error;
use ocelot::ot::mozzarella::cache::cacheinit::GenCache;
use ocelot::ot::mozzarella::cache::prover::CachedProver;
use ocelot::ot::mozzarella::cache::verifier::CachedVerifier;
use ocelot::ot::mozzarella::lpn::LLCode;
use ocelot::ot::mozzarella::{MozzarellaProver, MozzarellaVerifier};
use ocelot::quicksilver::{QuicksilverProver, QuicksilverVerifier};

use scuttlebutt::{channel::unix_channel_pair, Block, AesRng, AbstractChannel};
use scuttlebutt::channel::{Receivable, Sendable};
use scuttlebutt::ring::{NewRing, R64, Z2r, z2r};


fn setup_cache<RingT>() -> (CachedProver<RingT>, (CachedVerifier<RingT>, RingT))
    where
        RingT: NewRing,
        Standard: Distribution<RingT>,
{
    let mut rng = AesRng::from_seed(Default::default());
    let delta = rng.gen::<RingT>();
    let (prover_cache, verifier_cache) =
        GenCache::new_with_size(rng, delta, 5000);
    (prover_cache, (verifier_cache, delta))
}

fn generate_code<RingT>() -> LLCode<RingT>
    where
        RingT: NewRing,
        Standard: Distribution<RingT>,
{
    LLCode::<RingT>::from_seed(
        300,
        4992,
        ocelot::ot::mozzarella::CODE_D,
        Block::default(),
    )
}


fn run_quicksilver<RingT>() -> Result<(), Error>
    where
        RingT: NewRing + Receivable,
        for<'b> &'b RingT: Sendable,
        Standard: Distribution<RingT>,
{
    rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build_global()
        .unwrap();
    let (mut c1, mut c2) = unix_channel_pair();
    let (prover_cache, (verifier_cache, delta)) = setup_cache();

    let handle: JoinHandle<Result<(), Error>> = std::thread::spawn(move || {
        let code = generate_code::<RingT>();
        let mut moz_prover = MozzarellaProver::<RingT>::new(
            prover_cache,
            &code,
            300,
            16,
            312,
            false,
        );
        moz_prover.init(&mut c1).unwrap();

        let mut quicksilver_prover = QuicksilverProver::<RingT>::init(moz_prover);
        let (x1, z1) = quicksilver_prover.random(&mut c1)?;
        let (x2, z2) = quicksilver_prover.random(&mut c1)?;
        let (x3, z3) = quicksilver_prover.random(&mut c1)?;
        let (x4, z4) = quicksilver_prover.random(&mut c1)?;
        let (x5, z5) = quicksilver_prover.random(&mut c1)?;
        let (x6, z6) = quicksilver_prover.random(&mut c1)?;


        // todo: manual tests are the best (it even almost rhymes)
        let triples = vec![quicksilver_prover.multiply(&mut c1, (x1, z1), (x2, z2))?
                           , quicksilver_prover.multiply(&mut c1, (x3, z3), (x4, z4))?
                           , quicksilver_prover.multiply(&mut c1, (x5, z5), (x6, z6))?];
        quicksilver_prover.check_multiply(&mut c1, triples.as_slice());
        Ok(())
    });

    let code = generate_code::<RingT>();
    let rng = AesRng::new();
    let mut moz_verifier = MozzarellaVerifier::<RingT>::new(
        verifier_cache,
        &code,
        300,
        16,
        312,
        false,
    );
    moz_verifier.init(&mut c2, delta).unwrap();

    let mut quicksilver_verifier = QuicksilverVerifier::<RingT>::init(moz_verifier, delta);

    let y1 = quicksilver_verifier.random(&mut c2)?;
    let y2 = quicksilver_verifier.random(&mut c2)?;
    let y3 = quicksilver_verifier.random(&mut c2)?;
    let y4 = quicksilver_verifier.random(&mut c2)?;
    let y5 = quicksilver_verifier.random(&mut c2)?;
    let y6 = quicksilver_verifier.random(&mut c2)?;


    //let triples = vec![((y1, y2, y1_res))];
    let triples = vec![quicksilver_verifier.multiply(&mut c2, (y1, y2))?,
                       quicksilver_verifier.multiply(&mut c2, (y3, y4))?,
                       quicksilver_verifier.multiply(&mut c2, (y5, y6))?];
    quicksilver_verifier.check_multiply(&mut c2, rng, triples.as_slice());


    handle.join().unwrap();
    Ok(())
}



fn main() -> Result<(), Error> {
    run_quicksilver::<R64>()
}
