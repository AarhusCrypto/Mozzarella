// -*- mode: rust; -*-
//
// This file is part of ocelot.
// Copyright © 2020 Galois, Inc.
// See LICENSE for licensing information.

use ocelot::{
    ot::mozzarella::{
        cache::cacheinit::GenCache,
        init_lpn,
        MozzarellaProver,
        MozzarellaVerifier,
        REG_MAIN_K,
        REG_MAIN_LOG_SPLEN,
        REG_MAIN_T,
    },
    Error,
};
use rand::{rngs::OsRng, Rng};
use rayon;
use scuttlebutt::{channel::track_unix_channel_pair, ring::R64, AesRng, Block};
use std::{
    thread::{spawn, Builder, JoinHandle},
    time::Instant,
};

fn run() {
    let start = Instant::now();
    init_lpn();
    rayon::ThreadPoolBuilder::new()
        .num_threads(16)
        .stack_size(24 * 1024 * 1024)
        .build_global()
        .unwrap();

    let mut rng = OsRng;
    let fixed_key: Block = rng.gen();
    let delta: R64 = R64(fixed_key.extract_0_u64());
    let (prover_cache, verifier_cache) = GenCache::new::<_, REG_MAIN_K, REG_MAIN_T>(rng, delta);
    let (mut channel_v, mut channel_p) = track_unix_channel_pair();
    println!("Startup time (init): {:?}", start.elapsed());

    let mut moz_prover =
        MozzarellaProver::new(prover_cache, REG_MAIN_K, REG_MAIN_T, REG_MAIN_LOG_SPLEN);
    let mut moz_verifier =
        MozzarellaVerifier::new(verifier_cache, REG_MAIN_K, REG_MAIN_T, REG_MAIN_LOG_SPLEN);

    // Force the "main thread" to use a larger stack size of 16MB, as this is what is causing the stack overflows lol
    let prover_thread: JoinHandle<()> = Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let start = Instant::now();
            moz_prover.init(&mut channel_p).unwrap();
            println!("Prover time (init): {:?}", start.elapsed());

            let start = Instant::now();
            moz_prover.extend(&mut channel_p).unwrap();
            println!("Prover time (vole): {:?}", start.elapsed());

            // TODO: check that these two are correct (i.e. not swapped)
            println!(
                "Prover send communication (init): {:.2} Mb",
                channel_p.kilobits_read() / 1000.0
            );
            println!(
                "Prover receive communication (init): {:.2} Mb",
                channel_p.kilobits_written() / 1000.0
            );
        })
        .unwrap();

    let verifier_thread: JoinHandle<()> = Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let start = Instant::now();
            moz_verifier
                .init(&mut channel_v, &fixed_key.into())
                .unwrap();
            println!("Verifier time (init): {:?}", start.elapsed());

            let start = Instant::now();
            moz_verifier.extend(&mut channel_v).unwrap();
            println!("Verifier time (vole): {:?}", start.elapsed());
        })
        .unwrap();

    prover_thread.join().unwrap();
    verifier_thread.join().unwrap();
}

fn main() {
    println!("\nRing: R64 \n");
    run()
}
