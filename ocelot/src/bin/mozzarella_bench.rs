// -*- mode: rust; -*-
//
// This file is part of ocelot.
// Copyright © 2020 Galois, Inc.
// See LICENSE for licensing information.

use std::thread::{JoinHandle, spawn};
use scuttlebutt::{channel::track_unix_channel_pair, AesRng, Block};
use std::time::Instant;
use rand::Rng;
use rand::rngs::OsRng;
use ocelot::Error;
use ocelot::ot::mozzarella::{MozzarellaProver, MozzarellaVerifier, REG_MAIN_K, REG_MAIN_T};
use ocelot::ot::mozzarella::cache::cacheinit::GenCache;
use scuttlebutt::ring::R64;

const VOLE_ITER: usize = 40000;


fn run() {

    let (mut sender, mut receiver) = track_unix_channel_pair();

    let mut rng = OsRng;
    let fixed_key: Block = rng.gen();
    let delta: R64 = R64(fixed_key.extract_0_u64()); // fyfy, TODO
    let (prover_cache, verifier_cache) = GenCache::new::<_, REG_MAIN_K, REG_MAIN_T>(rng, delta);

    let handle: JoinHandle<Result<(), Error>> = spawn(move || {

        let start = Instant::now();
        // verifier init
        let mut moz_verifier = MozzarellaVerifier::init(delta, fixed_key.into(), verifier_cache);
        println!("Verifier time (init): {:?}", start.elapsed());


        let start = Instant::now();
        // verifier gen vole
        for _ in 0..VOLE_ITER {
            let _ = moz_verifier.vole(&mut sender, &mut rng).unwrap();
        }
        println!("Verifier time (vole): {:?}", start.elapsed());
        Ok(())

    });

    let mut rng = AesRng::new();
    let start = Instant::now();
    // prover init
    let mut moz_prover = MozzarellaProver::init(prover_cache);
    println!("Prover time (init): {:?}", start.elapsed());

    let start = Instant::now();

    // prover gen vole
    for _ in 0..VOLE_ITER {
        moz_prover.vole(&mut receiver, &mut rng).unwrap();
    }
    println!("Prover time (vole): {:?}", start.elapsed());


    // check that these two are correct (i.e. not swapped)
    println!(
        "Prover send communication (init): {:.2} Mb",
        receiver.kilobits_read() / 1000.0
    );
    println!(
        "Prover receive communication (init): {:.2} Mb",
        receiver.kilobits_written() / 1000.0
    );

    handle.join().unwrap();

}

fn main() {
    println!("\nRing: R64 \n");
    run()
}