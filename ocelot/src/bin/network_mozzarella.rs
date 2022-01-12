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
        REG_MAIN_CODE,
        REG_MAIN_K,
        REG_MAIN_LOG_SPLEN,
        REG_MAIN_T,
    },
    Error,
};

use clap::{App, Arg};
use rand::{rngs::OsRng, Rng, SeedableRng};
use rayon;
use scuttlebutt::{channel::track_unix_channel_pair, ring::R64, AesRng, Block, TrackChannel, SyncChannel};
use std::{
    thread::{spawn, Builder, JoinHandle},
    time::Instant,
};
use std::io::{BufReader, BufWriter};
use std::net::{TcpListener, TcpStream};

const DEFAULT_ADDR: &str = "127.0.0.1:5353";
const VERIFIER: &str = "VERIFIER";
const PROVER: &str = "PROVER";

fn run(whoami: &str,) -> std::io::Result<()> {
    let start = Instant::now();
    init_lpn();
    rayon::ThreadPoolBuilder::new()
        .num_threads(16)
        .stack_size(24 * 1024 * 1024)
        .build_global()
        .unwrap();

    // Use a seeded RNG as we need the same on both the prover and verifier
    let mut rng = AesRng::from_seed(Block::default());

    let fixed_key: Block = rng.gen();
    let delta: R64 = R64(fixed_key.extract_0_u64());
    let (prover_cache, verifier_cache) = GenCache::new::<_, REG_MAIN_K, REG_MAIN_T>(rng, delta);
    println!("Startup time (init): {:?}", start.elapsed());


    if whoami == VERIFIER {
        println!("Verifier started!");

        let listener = TcpListener::bind(DEFAULT_ADDR)?;

        match listener.accept() {
            Ok((stream_verifier, _addr)) => {

                let reader = BufReader::new(stream_verifier.try_clone().unwrap());
                let writer = BufWriter::new(stream_verifier);
                let mut channel_v: TrackChannel<
                    SyncChannel<BufReader<TcpStream>, BufWriter<TcpStream>>,
                > = TrackChannel::new(SyncChannel::new(reader, writer));

                let mut moz_verifier = MozzarellaVerifier::new(
                    verifier_cache,
                    &REG_MAIN_CODE,
                    REG_MAIN_K,
                    REG_MAIN_T,
                    REG_MAIN_LOG_SPLEN,
                );

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

                verifier_thread.join().unwrap();
            }
            Err(e) => println!("Couldn't get the client: {:?}", e),
        }

    } else {
        println!("Prover started!");

        let stream_prover = TcpStream::connect(DEFAULT_ADDR)?;
        let reader = BufReader::new(stream_prover.try_clone().unwrap());
        let writer = BufWriter::new(stream_prover);
        let mut channel_p = TrackChannel::new(SyncChannel::new(reader, writer));


        let mut moz_prover = MozzarellaProver::new(
            prover_cache,
            &REG_MAIN_CODE,
            REG_MAIN_K,
            REG_MAIN_T,
            REG_MAIN_LOG_SPLEN,
        );

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
        prover_thread.join().unwrap();
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    println!("\nRing: R64 \n");

    let matches = App::new("Mozzarella VOLE Generation")
        .version("1.0")
        .author("Alex Hansen, Lennart Braun")
        .about("")
        .arg(
            Arg::with_name("prover")
                .short('p')
                .long("prover")
                .help("set to be the prover")
                .required(false),
        ).get_matches();

    let whoami;
    if !matches.is_present("prover") {
        whoami = VERIFIER;
    } else {
        whoami = PROVER;
    }

    run(whoami)
}
