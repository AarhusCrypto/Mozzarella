use std::thread::{spawn, JoinHandle};

use rand::{rngs::OsRng, Rng, RngCore};

use scuttlebutt::{channel::unix_channel_pair, Block};
use ocelot::ot::mozzarella::spvole::{prover::Prover as spProver, verifier::Verifier as spVerifier};
use ocelot::ot::mozzarella::mozzarella::{prover::Prover as mozProver, verifier::Verifier as mozVerifier};
use ocelot::ot::{KosDeltaSender, Sender as OtSender, KosDeltaReceiver, Receiver as OtReceiver, FixedKeyInitializer};
use std::num::ParseIntError;
use std::sync::mpsc::channel;
use ocelot::Error;
use ocelot::ot::ferret::{FerretReceiver, FerretSender};
use ocelot::ot::mozzarella::{MozzarellaProver, MozzarellaVerifier};
use scuttlebutt::ring::R64;

const GEN_VOLE: usize = 1;

fn main() -> Result<(), Error>{
    //const K: usize = 589_760; // TODO: remove this eventually, when cache works
    const K: usize = 4; // TODO: remove this eventually, when cache works
    //const T: usize = 1_319; // TODO: remove this eventually, when cache works
    const T: usize = 1; // TODO: remove this eventually, when cache works
    let fixed_key: Block = OsRng.gen();
    let moz_delta: R64 = R64(fixed_key.extract_0_u64()); // fyfy, TODO
    println!("THE_DELTA:\t delta={}", moz_delta);

    // generate cached VOLEs
    let mut prover_cache: Vec<[(R64, R64); K]> = Vec::with_capacity(K);
    let mut verifier_cache: Vec<[R64; K]> = Vec::with_capacity(K);
    // only produce K currently
    let mut single_prover_cache: [(R64, R64); K] = [(R64::default(), R64::default()); K];
    let mut single_verifier_cache: [R64; K] = [R64::default(); K];
    for i in 0..K {
        let a1 = R64(OsRng.next_u64());
        let b1 = R64(OsRng.next_u64());
        let mut tmp = a1;
        tmp *= moz_delta;
        let mut c1 = tmp;
        c1 += b1;
        single_verifier_cache[i] = b1;
        single_prover_cache[i] = (a1, c1);
    }

    prover_cache.push(single_prover_cache.into());
    verifier_cache.push(single_verifier_cache.into());


    // generate base voles for spsvole
    let mut prover_base: [((R64, R64), (R64, R64)); T] = [((R64::default(), R64::default()), (R64::default(), R64::default())); T];
    let mut verifier_base: [(R64, R64); T] = [(R64::default(), R64::default()); T];
    for i in 0..T {
        let a1 = R64(OsRng.next_u64());
        let b1 = R64(OsRng.next_u64());
        let mut tmp = a1;
        tmp *= moz_delta;
        let mut c1 = tmp;
        c1 += b1;

        let a2 = R64(OsRng.next_u64());
        let b2 = R64(OsRng.next_u64());
        let mut tmp = a2;
        tmp *= moz_delta;
        let mut c2 = tmp;
        c2 += b2;

        verifier_base[i] = (b1, b2);
        prover_base[i] = ((a1, c1), (a2, c2));
    }


    let (mut c1, mut c2) = unix_channel_pair();


    let handle: JoinHandle<Result<(), Error>> = spawn(move || {
        let mut moz_verifier = MozzarellaVerifier::init(moz_delta, fixed_key.into());
        for _ in 0..GEN_VOLE {
            moz_verifier.vole(&mut c1, &mut OsRng, &mut verifier_base, &mut verifier_cache)?;
        }
        return Ok(());
    });


    let mut moz_prover = MozzarellaProver::init();

    for _ in 0..GEN_VOLE {
        moz_prover.vole(&mut c2, &mut OsRng, &mut prover_base, &mut prover_cache)?;
    }
    handle.join().unwrap();
    return Ok(());




/*
    let tmp: Block = OsRng.gen();
    let tester: Block = OsRng.gen();
    let delta: R64 = R64(tmp.extract_0_u64()); // fyfy, TODO

    // hardcode the two extend calls that we'll need later (since we can't do base vole yet)
    let a1 = R64(OsRng.next_u64());
    let b1 = R64(OsRng.next_u64());
    let mut tmp = a1;
    //println!("TEST:\t a1 = {}", a1);
    tmp *= delta;
    //println!("TEST:\t a1*delta {}", a1);
    let mut c1 = tmp;
    c1 += b1;
    //println!("TEST:\t c1 {}", c1);


    let a2 = R64(OsRng.next_u64());
    let b2 = R64(OsRng.next_u64());
    tmp = a2;
    tmp *= delta;
    let mut c2 = tmp;
    c2 += b2;

    let mut prover_base_voles = vec!((a1, c1), (a2, c2));
    let mut verifier_base_voles = vec!(b1, b2);
    let (mut c1, mut c2) = unix_channel_pair();


    let handle: JoinHandle<Result<(), Error>> = spawn(move || {



        let mut kos18_sender = KosDeltaSender::init_fixed_key(&mut c1, tester.into(), &mut OsRng)?;


        let mut verifier_ = verifier::Verifier::init(delta);
        for _ in 0..GEN_COTS {
            verifier_.extend(&mut c1, &mut OsRng, 1, &mut kos18_sender, &mut verifier_base_voles)?;
        }
        return Ok(());
    });

    let mut prover_ = prover::Prover::init();
    let mut kos18_receiver = KosDeltaReceiver::init(&mut c2, &mut OsRng)?;

    for n in 0..GEN_COTS {
        prover_.extend(&mut c2, &mut OsRng, 1, &mut kos18_receiver, &mut prover_base_voles, 4 as usize)?;
    }
    handle.join().unwrap();
    return Ok(());





    let (mut c1, mut c2) = unix_channel_pair();

    let handle = spawn(move || {
        let delta: Block = OsRng.gen();
        let mut sender = FerretSender::init(delta, &mut c1, &mut OsRng).unwrap();
        for _ in 0..GEN_COTS {
            let _output: Block = sender.cot(&mut c1, &mut OsRng).unwrap();
        }
    });

    let mut receiver = FerretReceiver::init(&mut c2, &mut OsRng).unwrap();
    for n in 0..GEN_COTS {
        let _cot: (bool, Block) = receiver.cot(&mut c2, &mut OsRng).unwrap();
        println!("bool: {}", _cot.0);
        println!("block: {}", _cot.1);
    }
    handle.join().unwrap();
    return Ok(());
    */
}