// -*- mode: rust; -*-
//
// This file is part of ocelot.
// Copyright © 2020 Galois, Inc.
// See LICENSE for licensing information.

use clap::{ArgEnum, ErrorKind, IntoApp, Parser};
use ocelot::{
    ot::mozzarella::{
        cache::{cacheinit::GenCache, prover::CachedProver, verifier::CachedVerifier},
        lpn::LLCode,
        MozzarellaProver,
        MozzarellaVerifier,
        CODE_D,
    },
    Error,
};
use rand::{Rng, SeedableRng};
use rayon;
use scuttlebutt::{
    channel::track_unix_channel_pair,
    ring::R64,
    AbstractChannel,
    AesRng,
    Block,
    SyncChannel,
    TrackChannel,
};
use std::{
    fmt,
    io::{BufReader, BufWriter},
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread,
    time::Instant,
};

#[derive(Debug, Clone, ArgEnum)]
enum Role {
    Prover,
    Verifier,
    Both,
}

#[derive(Debug, Parser)]
struct NetworkOptions {
    #[clap(short, long)]
    listen: bool,
    #[clap(short, long, default_value = "localhost")]
    host: String,
    #[clap(short, long, default_value_t = 1337)]
    port: u16,
}

#[derive(Debug, Copy, Clone, Parser)]
struct LpnParameters {
    #[clap(short = 'K', long)]
    base_vole_size: usize,
    #[clap(short = 'N', long)]
    extension_size: usize,
    #[clap(short = 'T', long)]
    num_noise_coordinates: usize,
}

impl LpnParameters {
    fn log2(x: usize) -> usize {
        assert!(x.is_power_of_two());
        let mut log = 0;
        let mut x = x;
        while x > 1 {
            log += 1;
            x >>= 1;
        }
        log
    }

    fn get_block_size(&self) -> usize {
        self.extension_size / self.num_noise_coordinates
    }

    fn get_log_block_size(&self) -> usize {
        Self::log2(self.get_block_size())
    }

    fn get_required_cache_size(&self) -> usize {
        self.base_vole_size + 2 * self.num_noise_coordinates
    }

    pub fn validate(&self) -> bool {
        self.base_vole_size > 0
            && self.extension_size > 0
            && self.num_noise_coordinates > 0
            && self.extension_size % self.num_noise_coordinates == 0
            && self.get_block_size().is_power_of_two()
    }
}

impl fmt::Display for LpnParameters {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "LPN[K = {}, N = {}, T = {}, M = {} = 2^{}]",
            self.base_vole_size,
            self.extension_size,
            self.num_noise_coordinates,
            self.get_block_size(),
            self.get_log_block_size(),
        )
    }
}

#[derive(Debug, Parser)]
#[clap(
    name = "Mozzarella VOLE Generation Benchmark",
    author = "Alex Hansen, Lennart Braun",
    version = "0.1"
)]
struct Options {
    #[clap(short, long, arg_enum)]
    role: Role,

    #[clap(flatten, help_heading = "LPN parameters")]
    lpn_parameters: LpnParameters,

    #[clap(flatten, help_heading = "network options")]
    network_options: NetworkOptions,

    #[clap(short, long, default_value_t = 0)]
    threads: usize,

    #[clap(short, long)]
    verbose: bool,
}

type NetworkChannel = TrackChannel<SyncChannel<BufReader<TcpStream>, BufWriter<TcpStream>>>;

fn connect(host: &str, port: u16) -> Result<NetworkChannel, Error> {
    let socket = TcpStream::connect((host, port))?;
    let reader = BufReader::new(socket.try_clone()?);
    let writer = BufWriter::new(socket);
    let channel = TrackChannel::new(SyncChannel::new(reader, writer));
    Ok(channel)
}

fn listen(host: &str, port: u16) -> Result<NetworkChannel, Error> {
    let listener = TcpListener::bind((host, port))?;
    let (socket, _addr) = listener.accept()?;
    let reader = BufReader::new(socket.try_clone()?);
    let writer = BufWriter::new(socket);
    let channel = TrackChannel::new(SyncChannel::new(reader, writer));
    Ok(channel)
}

fn setup_network(options: &NetworkOptions) -> Result<NetworkChannel, Error> {
    if options.listen {
        listen(&options.host, options.port)
    } else {
        connect(&options.host, options.port)
    }
}

fn setup_cache(lpn_parameters: &LpnParameters) -> (CachedProver, (CachedVerifier, Block)) {
    let mut rng = AesRng::from_seed(Default::default());
    let fixed_key: Block = rng.gen();
    let delta: R64 = R64(fixed_key.extract_0_u64());
    let (prover_cache, verifier_cache) =
        GenCache::new_with_size(rng, delta, lpn_parameters.get_required_cache_size());
    (prover_cache, (verifier_cache, fixed_key))
}

fn generate_code(lpn_parameters: &LpnParameters) -> LLCode {
    LLCode::from_seed(
        lpn_parameters.base_vole_size,
        lpn_parameters.extension_size,
        CODE_D,
        Block::default(),
    )
}

fn run_prover<C: AbstractChannel>(
    channel: &mut C,
    lpn_parameters: LpnParameters,
    code: &LLCode,
    cache: CachedProver,
) {
    let mut moz_prover = MozzarellaProver::new(
        cache,
        code,
        lpn_parameters.base_vole_size,
        lpn_parameters.num_noise_coordinates,
        lpn_parameters.get_log_block_size(),
    );
    let start = Instant::now();
    moz_prover.init(channel).unwrap();
    println!("Prover time (init): {:?}", start.elapsed());

    let start = Instant::now();
    moz_prover.extend(channel).unwrap();
    println!("Prover time (vole): {:?}", start.elapsed());
}

fn run_verifier<C: AbstractChannel>(
    channel: &mut C,
    lpn_parameters: LpnParameters,
    code: &LLCode,
    cache: CachedVerifier,
    fixed_key: Block,
) {
    let mut moz_verifier = MozzarellaVerifier::new(
        cache,
        code,
        lpn_parameters.base_vole_size,
        lpn_parameters.num_noise_coordinates,
        lpn_parameters.get_log_block_size(),
    );
    let start = Instant::now();
    moz_verifier.init(channel, &fixed_key.into()).unwrap();
    println!("Verifier time (init): {:?}", start.elapsed());

    let start = Instant::now();
    moz_verifier.extend(channel).unwrap();
    println!("Verifier time (vole): {:?}", start.elapsed());
}

fn run() {
    let options = Options::parse();
    let mut app = Options::into_app();

    println!("LPN Parameters: {}", options.lpn_parameters);
    if !options.lpn_parameters.validate() {
        app.error(
            ErrorKind::ArgumentConflict,
            "Invalid / not-supported LPN parameters",
        )
        .exit();
    }
    assert!(options.lpn_parameters.validate());
    println!("{:?}", options);

    let t_start = Instant::now();
    let code = generate_code(&options.lpn_parameters);
    rayon::ThreadPoolBuilder::new()
        .num_threads(options.threads)
        .build_global()
        .unwrap();
    let (prover_cache, (verifier_cache, verifier_fixed_key)) = setup_cache(&options.lpn_parameters);
    println!("Startup time: {:?}", t_start.elapsed());

    match options.role {
        Role::Both => {
            let (mut channel_v, mut channel_p) = track_unix_channel_pair();
            let lpn_parameters_p = options.lpn_parameters;
            let lpn_parameters_v = options.lpn_parameters;
            let code_p = Arc::new(code);
            let code_v = code_p.clone();
            let prover_thread = thread::spawn(move || {
                run_prover(&mut channel_p, lpn_parameters_p, &code_p, prover_cache)
            });
            let verifier_thread = thread::spawn(move || {
                run_verifier(
                    &mut channel_v,
                    lpn_parameters_v,
                    &code_v,
                    verifier_cache,
                    verifier_fixed_key,
                )
            });
            prover_thread.join().unwrap();
            verifier_thread.join().unwrap();
        }
        role => {
            let mut channel = {
                match setup_network(&options.network_options) {
                    Ok(channel) => channel,
                    Err(e) => {
                        eprintln!("Network connection failed: {}", e.to_string());
                        return;
                    }
                }
            };
            match role {
                Role::Prover => {
                    run_prover(&mut channel, options.lpn_parameters, &code, prover_cache)
                }
                Role::Verifier => run_verifier(
                    &mut channel,
                    options.lpn_parameters,
                    &code,
                    verifier_cache,
                    verifier_fixed_key,
                ),
                _ => panic!("can't happen"),
            }
            println!("sent data: {:.2} MiB", channel.kilobytes_written() / 1024.0);
            println!(
                "received data: {:.2} MiB",
                channel.kilobytes_read() / 1024.0
            );
        }
    }
}

fn main() {
    println!("\nRing: R64 \n");
    run()
}
