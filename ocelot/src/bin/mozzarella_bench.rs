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
        MozzarellaProver, MozzarellaProverStats, MozzarellaVerifier, MozzarellaVerifierStats,
        CODE_D,
    },
    tools::BenchmarkMetaData,
    Error,
};
use rand::{
    distributions::{Distribution, Standard},
    Rng, SeedableRng,
};
use rayon;
use scuttlebutt::{
    channel::{track_unix_channel_pair, Receivable, Sendable},
    ring::{z2r, NewRing, R64, RX},
    AbstractChannel, AesRng, Block, SyncChannel, TrackChannel,
};
use serde::Serialize;
use serde_json;
use std::{
    fmt,
    io::{BufReader, BufWriter},
    net::{TcpListener, TcpStream},
    string::ToString,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, ArgEnum)]
enum Party {
    Prover,
    Verifier,
    Both,
}

impl fmt::Display for Party {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Party::Prover => write!(f, "Prover"),
            Party::Verifier => write!(f, "Verifier"),
            Party::Both => write!(f, "Both"),
        }
    }
}

#[derive(Debug, Clone, Parser, Serialize)]
struct NetworkOptions {
    #[clap(short, long)]
    listen: bool,
    #[clap(short, long, default_value = "localhost")]
    host: String,
    #[clap(short, long, default_value_t = 1337)]
    port: u16,
    #[clap(long, default_value_t = 100)]
    connect_timeout_seconds: usize,
}

#[derive(Debug, Copy, Clone, Parser, Serialize)]
struct LpnParameters {
    #[clap(short = 'K', long)]
    base_vole_size: usize,
    #[clap(short = 'N', long)]
    extension_size: usize,
    #[clap(short = 'T', long)]
    num_noise_coordinates: usize,
}

impl LpnParameters {
    fn recompute_extension_size(&mut self) {
        assert!(self.num_noise_coordinates > 0);
        // increase extension_size s.t. it is a multiple of the number of noise coordinates
        // ceil(extension_size / num_noise_coordinates)
        let block_size = 1 + (self.extension_size - 1) / self.num_noise_coordinates;
        // recompute extension size to be a multiple of the block size
        self.extension_size = block_size * self.num_noise_coordinates;
    }

    fn get_block_size(&self) -> usize {
        1 + (self.extension_size - 1) / self.num_noise_coordinates
    }

    fn get_required_cache_size(&self) -> usize {
        self.base_vole_size + 2 * self.num_noise_coordinates
    }

    fn get_vole_output_size(&self) -> usize {
        assert!(self.extension_size >= self.get_required_cache_size());
        self.extension_size - self.get_required_cache_size()
    }

    pub fn validate(&self) -> bool {
        self.base_vole_size > 0
            && self.extension_size > 0
            && self.num_noise_coordinates > 0
            && self.extension_size % self.num_noise_coordinates == 0
    }
}

impl fmt::Display for LpnParameters {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "LPN[K = {}, N = {}, T = {}, M = {}]",
            self.base_vole_size,
            self.extension_size,
            self.num_noise_coordinates,
            self.get_block_size(),
        )
    }
}

#[derive(Debug, Clone, ArgEnum)]
enum RingParameter {
    R64,
    R72,
    // R78,
    R104,
    R104_U192,
    R104_U256,
    // R110,
    // R112,
    // R118,
    // R119,
    R144,
    // R150,
    // R151,
    // R203,
    // R224,
    // R231,
    RX,
}

impl fmt::Display for RingParameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            RingParameter::R64 => write!(f, "R64"),
            RingParameter::R72 => write!(f, "R72"),
            RingParameter::R104 => write!(f, "R104"),
            RingParameter::R104_U192 => write!(f, "R104_U192"),
            RingParameter::R104_U256 => write!(f, "R104_U256"),
            RingParameter::R144 => write!(f, "R144"),
            RingParameter::RX => write!(f, "RX"),
        }
    }
}

#[derive(Debug, Parser)]
#[clap(
    name = "Mozzarella VOLE Generation Benchmark",
    author = "Alex Hansen, Lennart Braun",
    version = "0.1"
)]
struct Options {
    #[clap(short = 'P', long, arg_enum)]
    party: Party,

    #[clap(short = 'R', long, arg_enum, default_value_t = RingParameter::R64)]
    ring: RingParameter,

    #[clap(flatten, help_heading = "LPN parameters")]
    lpn_parameters: LpnParameters,

    #[clap(flatten, help_heading = "network options")]
    network_options: NetworkOptions,

    #[clap(short, long, default_value_t = 0)]
    threads: usize,

    #[clap(short, long, default_value_t = 1)]
    repetitions: usize,

    #[clap(short, long)]
    nightly: bool,

    #[clap(short, long)]
    json: bool,

    #[clap(short, long)]
    verbose: bool,
}

#[derive(Clone, Debug, Serialize)]
enum PartyStats {
    ProverStats(MozzarellaProverStats),
    VerifierStats(MozzarellaVerifierStats),
}

#[derive(Clone, Debug, Default, Serialize)]
struct StatTuple {
    pub n: usize,
    pub ns_avg: f64,
    pub ns_avg_per_vole: f64,
    pub ns_median: f64,
    pub ns_median_per_vole: f64,
    pub ns_stddev: f64,
    pub ns_stddev_per_vole: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
struct RunTimeStats {
    pub init_run_times: Vec<Duration>,
    pub extend_run_times: Vec<Duration>,
    pub init_stats: StatTuple,
    pub extend_stats: StatTuple,
    pub kilobytes_sent: f64,
    pub kilobytes_received: f64,
    pub party_stats: Vec<PartyStats>,
}

impl RunTimeStats {
    fn analyse_times(times: &[Duration], num_voles: usize) -> StatTuple {
        let n = times.len();
        assert!(n > 0);
        let mut ns: Vec<u128> = times.iter().map(|d| d.as_nanos()).collect();
        ns.sort_unstable();
        let ns_avg = ns.iter().sum::<u128>() as f64 / n as f64;
        let ns_median = ns[n / 2] as f64;
        let ns_stddev = if n > 1 {
            ns.iter()
                .map(|x| (*x as f64 - ns_avg).powf(2f64))
                .sum::<f64>()
                / (n - 1) as f64
        } else {
            f64::NAN
        };
        StatTuple {
            n,
            ns_avg,
            ns_avg_per_vole: ns_avg / num_voles as f64,
            ns_median,
            ns_median_per_vole: ns_median / num_voles as f64,
            ns_stddev,
            ns_stddev_per_vole: ns_stddev / num_voles as f64,
        }
    }

    pub fn compute_statistics(&mut self, num_voles: usize) {
        self.init_stats = Self::analyse_times(&self.init_run_times, num_voles);
        self.extend_stats = Self::analyse_times(&self.extend_run_times, num_voles);
    }
}

#[derive(Clone, Debug, Serialize)]
struct BenchmarkResult {
    pub run_time_stats: RunTimeStats,
    pub repetitions: usize,
    pub party: String,
    pub ring: String,
    pub threads: usize,
    pub network_options: NetworkOptions,
    pub lpn_parameters: LpnParameters,
    pub meta_data: BenchmarkMetaData,
}

impl BenchmarkResult {
    pub fn new(options: &Options) -> Self {
        Self {
            run_time_stats: RunTimeStats {
                init_run_times: Vec::with_capacity(options.repetitions),
                extend_run_times: Vec::with_capacity(options.repetitions),
                init_stats: Default::default(),
                extend_stats: Default::default(),
                kilobytes_sent: 0f64,
                kilobytes_received: 0f64,
                party_stats: Vec::with_capacity(options.repetitions),
            },
            repetitions: 0,
            party: options.party.to_string(),
            ring: options.ring.to_string(),
            threads: options.threads,
            network_options: options.network_options.clone(),
            lpn_parameters: options.lpn_parameters,
            meta_data: BenchmarkMetaData::collect(),
        }
    }
}

type NetworkChannel = TrackChannel<SyncChannel<BufReader<TcpStream>, BufWriter<TcpStream>>>;

fn connect(host: &str, port: u16, timeout_seconds: usize) -> Result<NetworkChannel, Error> {
    fn connect_socket(host: &str, port: u16, timeout_seconds: usize) -> Result<TcpStream, Error> {
        for _ in 0..(10 * timeout_seconds) {
            match TcpStream::connect((host, port)) {
                Ok(socket) => return Ok(socket),
                Err(_) => (),
            }
            thread::sleep(Duration::from_millis(100));
        }
        match TcpStream::connect((host, port)) {
            Ok(socket) => return Ok(socket),
            Err(e) => Err(Error::IoError(e)),
        }
    }
    let socket = connect_socket(host, port, timeout_seconds)?;
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
        connect(&options.host, options.port, options.connect_timeout_seconds)
    }
}

fn setup_cache<RingT>(
    lpn_parameters: &LpnParameters,
) -> (CachedProver<RingT>, (CachedVerifier<RingT>, RingT))
where
    RingT: NewRing,
    Standard: Distribution<RingT>,
{
    let mut rng = AesRng::from_seed(Default::default());
    let delta = rng.gen::<RingT>();
    let (prover_cache, verifier_cache) =
        GenCache::new_with_size(rng, delta, lpn_parameters.get_required_cache_size());
    (prover_cache, (verifier_cache, delta))
}

fn generate_code<RingT>(lpn_parameters: &LpnParameters) -> LLCode<RingT>
where
    RingT: NewRing,
    Standard: Distribution<RingT>,
{
    LLCode::<RingT>::from_seed(
        lpn_parameters.base_vole_size,
        lpn_parameters.extension_size,
        CODE_D,
        Block::default(),
    )
}

fn run_prover<RingT, C: AbstractChannel>(
    channel: &mut C,
    lpn_parameters: LpnParameters,
    code: &LLCode<RingT>,
    cache: CachedProver<RingT>,
    nightly: bool,
) -> (Duration, Duration, PartyStats)
where
    RingT: NewRing + Receivable,
    for<'b> &'b RingT: Sendable,
    Standard: Distribution<RingT>,
{
    let mut moz_prover = MozzarellaProver::<RingT>::new(
        cache,
        code,
        lpn_parameters.base_vole_size,
        lpn_parameters.num_noise_coordinates,
        lpn_parameters.get_block_size(),
        nightly,
    );
    let t_start = Instant::now();
    moz_prover.init(channel).unwrap();
    let run_time_init = t_start.elapsed();

    let t_start = Instant::now();
    moz_prover.extend(channel).unwrap();
    let run_time_extend = t_start.elapsed();

    (
        run_time_init,
        run_time_extend,
        PartyStats::ProverStats(moz_prover.get_stats()),
    )
}

fn run_verifier<RingT, C: AbstractChannel>(
    channel: &mut C,
    lpn_parameters: LpnParameters,
    code: &LLCode<RingT>,
    cache: CachedVerifier<RingT>,
    delta: RingT,
    nightly: bool,
) -> (Duration, Duration, PartyStats)
where
    RingT: NewRing + Receivable,
    for<'b> &'b RingT: Sendable,
    Standard: Distribution<RingT>,
{
    let mut moz_verifier = MozzarellaVerifier::<RingT>::new(
        cache,
        code,
        lpn_parameters.base_vole_size,
        lpn_parameters.num_noise_coordinates,
        lpn_parameters.get_block_size(),
        nightly,
    );
    let t_start = Instant::now();
    moz_verifier.init(channel, delta).unwrap();
    let run_time_init = t_start.elapsed();

    let t_start = Instant::now();
    moz_verifier.extend(channel).unwrap();
    let run_time_extend = t_start.elapsed();

    (
        run_time_init,
        run_time_extend,
        PartyStats::VerifierStats(moz_verifier.get_stats()),
    )
}

fn run_benchmark<RingT>(options: &Options)
where
    RingT: NewRing + Receivable,
    for<'b> &'b RingT: Sendable,
    Standard: Distribution<RingT>,
{
    let t_start = Instant::now();
    let code = generate_code::<RingT>(&options.lpn_parameters);
    rayon::ThreadPoolBuilder::new()
        .num_threads(options.threads)
        .build_global()
        .unwrap();
    let (prover_cache, (verifier_cache, delta)) = setup_cache(&options.lpn_parameters);
    if !options.json {
        println!("Startup time: {:?}", t_start.elapsed());
    }

    let mut results = BenchmarkResult::new(&options);

    match &options.party {
        Party::Both => {
            let (mut channel_v, mut channel_p) = track_unix_channel_pair();
            let lpn_parameters_p = options.lpn_parameters;
            let lpn_parameters_v = options.lpn_parameters;
            let code_p = Arc::new(code);
            let code_v = code_p.clone();
            let repetitions = options.repetitions;
            let nightly = options.nightly;
            let mut results_p = BenchmarkResult::new(&options);
            let mut results_v = results_p.clone();
            let prover_thread = thread::spawn(move || {
                for _ in 0..repetitions {
                    let (run_time_init, run_time_extend, party_stats) = run_prover::<RingT, _>(
                        &mut channel_p,
                        lpn_parameters_p,
                        &code_p,
                        prover_cache.clone(),
                        nightly,
                    );
                    results_p.run_time_stats.init_run_times.push(run_time_init);
                    results_p
                        .run_time_stats
                        .extend_run_times
                        .push(run_time_extend);
                    results_p.run_time_stats.party_stats.push(party_stats);
                    results_p.repetitions += 1;
                    if results_p.repetitions == 1 {
                        results_p.run_time_stats.kilobytes_sent = channel_p.kilobytes_written();
                        results_p.run_time_stats.kilobytes_received = channel_p.kilobytes_read();
                    }
                }
                results_p
            });
            let verifier_thread = thread::spawn(move || {
                for _ in 0..repetitions {
                    let (run_time_init, run_time_extend, party_stats) = run_verifier::<RingT, _>(
                        &mut channel_v,
                        lpn_parameters_v,
                        &code_v,
                        verifier_cache.clone(),
                        delta,
                        nightly,
                    );
                    results_v.run_time_stats.init_run_times.push(run_time_init);
                    results_v
                        .run_time_stats
                        .extend_run_times
                        .push(run_time_extend);
                    results_v.run_time_stats.party_stats.push(party_stats);
                    results_v.repetitions += 1;
                    if results_v.repetitions == 1 {
                        results_v.run_time_stats.kilobytes_sent = channel_v.kilobytes_written();
                        results_v.run_time_stats.kilobytes_received = channel_v.kilobytes_read();
                    }
                }
                results_v
            });
            let mut results_p = prover_thread.join().unwrap();
            results_p
                .run_time_stats
                .compute_statistics(options.lpn_parameters.get_vole_output_size());
            let mut results_v = verifier_thread.join().unwrap();
            results_v
                .run_time_stats
                .compute_statistics(options.lpn_parameters.get_vole_output_size());
            if options.json {
                println!("{}", serde_json::to_string_pretty(&results_p).unwrap());
                println!("{}", serde_json::to_string_pretty(&results_v).unwrap());
            } else {
                println!("results prover: {:?}", results_p);
                println!("results verifier: {:?}", results_v);
            }
        }
        party => {
            let mut channel = {
                match setup_network(&options.network_options) {
                    Ok(channel) => channel,
                    Err(e) => {
                        eprintln!("Network connection failed: {}", e.to_string());
                        return;
                    }
                }
            };
            for _ in 0..options.repetitions {
                let (run_time_init, run_time_extend, party_stats) = match party {
                    Party::Prover => run_prover::<RingT, _>(
                        &mut channel,
                        options.lpn_parameters,
                        &code,
                        prover_cache.clone(),
                        options.nightly,
                    ),
                    Party::Verifier => run_verifier::<RingT, _>(
                        &mut channel,
                        options.lpn_parameters,
                        &code,
                        verifier_cache.clone(),
                        delta,
                        options.nightly,
                    ),
                    _ => panic!("can't happen"),
                };
                results.run_time_stats.init_run_times.push(run_time_init);
                results
                    .run_time_stats
                    .extend_run_times
                    .push(run_time_extend);
                results.run_time_stats.party_stats.push(party_stats);
                results.repetitions += 1;
                if results.repetitions == 1 {
                    results.run_time_stats.kilobytes_sent = channel.kilobytes_written();
                    results.run_time_stats.kilobytes_received = channel.kilobytes_read();
                }
                if !options.json {
                    println!("{:?} time (init): {:?}", options.party, run_time_init);
                    println!("{:?} time (vole): {:?}", options.party, run_time_extend);
                    println!("sent data: {:.2} MiB", channel.kilobytes_written() / 1024.0);
                    println!(
                        "received data: {:.2} MiB",
                        channel.kilobytes_read() / 1024.0
                    );
                }
            }
            results
                .run_time_stats
                .compute_statistics(options.lpn_parameters.get_vole_output_size());
            if options.json {
                println!("{}", serde_json::to_string_pretty(&results).unwrap());
            } else {
                println!("results: {:?}", results);
            }
        }
    }
}

fn run() {
    let mut options = Options::parse();
    let mut app = Options::into_app();

    if !options.json {
        println!("LPN Parameters: {}", options.lpn_parameters);
    }
    options.lpn_parameters.recompute_extension_size();
    if !options.lpn_parameters.validate() {
        app.error(
            ErrorKind::ArgumentConflict,
            "Invalid / not-supported LPN parameters",
        )
        .exit();
    }
    assert!(options.lpn_parameters.validate());
    if !options.json {
        println!("{:?}", options);
    }

    match options.ring {
        RingParameter::R64 => run_benchmark::<R64>(&options),
        RingParameter::R72 => run_benchmark::<z2r::R72>(&options),
        RingParameter::R104 => run_benchmark::<z2r::R104>(&options),
        RingParameter::R104_U192 => run_benchmark::<z2r::Z2rU192<104>>(&options),
        RingParameter::R104_U256 => run_benchmark::<z2r::Z2rU256<104>>(&options),
        RingParameter::R144 => run_benchmark::<z2r::R144>(&options),
        RingParameter::RX => run_benchmark::<RX>(&options),
        // _ => (),
    }
}

fn main() {
    run()
}
