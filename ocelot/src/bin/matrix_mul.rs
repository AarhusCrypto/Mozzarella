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
        GenCache::new_with_size(rng, delta, 150000);
    (prover_cache, (verifier_cache, delta))
}

fn generate_code<RingT>() -> LLCode<RingT>
    where
        RingT: NewRing,
        Standard: Distribution<RingT>,
{
    LLCode::<RingT>::from_seed(
        2000,
        65536,
        ocelot::ot::mozzarella::CODE_D,
        Block::default(),
    )
}

fn run_matrix_mul_benchmark<RingT>() -> Result<(), Error>
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

    const DIM: usize = 50;
    let handle: JoinHandle<Result<(), Error>> = std::thread::spawn(move || {
        let mut rng = AesRng::from_seed(Block::default());

        let code = generate_code::<RingT>();
        let mut moz_prover = MozzarellaProver::<RingT>::new(
            prover_cache,
            &code,
            2000,
            16,
            4096,
            false,
        );
        moz_prover.init(&mut c1).unwrap();

        let mut quicksilver_prover = QuicksilverProver::<RingT>::init(moz_prover);

        // sample the matrices A,B and define C incredibly naïvely lol
        let mut A = [[(RingT::default(), RingT::default()); DIM]; DIM];
        let mut B = [[(RingT::default(), RingT::default()); DIM]; DIM];
        //let mut C = [[(RingT::default(), RingT::default()); DIM]; DIM];

        for i in 0..DIM {
            for j in 0..DIM {
                let tmp_1 = rng.gen::<RingT>();
                A[i][j] = quicksilver_prover.input(&mut c1, tmp_1)?;
                let tmp_2 = rng.gen::<RingT>();
                B[i][j] = quicksilver_prover.input(&mut c1, tmp_2)?;
            }
        }
        let mut triples: Vec<((RingT, RingT),
            (RingT, RingT),
            (RingT, RingT))> = Vec::new();

        for row in 0..DIM {
            for col in 0..DIM {
                let mut tmp: RingT = RingT::default();
                for i in 0..DIM {
                    let out = quicksilver_prover.multiply(&mut c1,
                                                A[row][i],
                                                B[i][col])?;
                    tmp += out.2.0;
                    triples.push(out);
                }
                //C[row][col] = tmp;
            }
        }

        quicksilver_prover.check_multiply(&mut c1, triples.as_slice());
        Ok(())
    });

    let code = generate_code::<RingT>();
    let mut rng = AesRng::from_seed(Block::default());

    let mut moz_verifier = MozzarellaVerifier::<RingT>::new(
        verifier_cache,
        &code,
        2000,
        16,
        4096,
        false,
    );
    moz_verifier.init(&mut c2, delta).unwrap();

    let mut quicksilver_verifier = QuicksilverVerifier::<RingT>::init(moz_verifier, delta);



    // sample the matrices A,B and define C incredibly naïvely lol
    let mut A = [[RingT::default(); DIM]; DIM];
    let mut B = [[RingT::default(); DIM]; DIM];
    //let mut C = [[RingT::default(); DIM]; DIM];

    let mut triples: Vec<(RingT,
                      RingT,
                      RingT)> = Vec::new();

    for i in 0..DIM {
        for j in 0..DIM {
            A[i][j] = quicksilver_verifier.input(&mut c2)?;
            B[i][j] = quicksilver_verifier.input(&mut c2)?;
        }
    }

    // todo: Don't actually need C
    for row in 0..DIM {
        for col in 0..DIM {
            for i in 0..DIM {
                let out = quicksilver_verifier.multiply(&mut c2,
                                                        (A[row][i], B[i][col]))?;
                //C[row][col] = out.2;
                triples.push(out);
            }
        }
    }



    quicksilver_verifier.check_multiply(&mut c2, rng, triples.as_slice());


    handle.join().unwrap();
    Ok(())
}



fn main() -> Result<(), Error> {
    run_matrix_mul_benchmark::<R64>()
}
