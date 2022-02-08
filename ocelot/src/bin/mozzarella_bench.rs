// -*- mode: rust; -*-
//
// This file is part of ocelot.
// Copyright © 2020 Galois, Inc.
// See LICENSE for licensing information.

use clap::{ErrorKind, IntoApp, Parser};
use ocelot::{
    benchmark_tools::{
        generate_code, setup_cache, setup_network, LpnParameters, NetworkOptions, Party,
        RingParameter,
    },
    ot::mozzarella::{
        cache::{prover::CachedProver, verifier::CachedVerifier},
        lpn::LLCode,
        MozzarellaProver, MozzarellaProverStats, MozzarellaVerifier, MozzarellaVerifierStats,
    },
    tools::BenchmarkMetaData,
};
use rand::distributions::{Distribution, Standard};
use rayon;
use scuttlebutt::{
    channel::{track_unix_channel_pair, Receivable, Sendable},
    ring::{z2r, NewRing, R64, RX},
    AbstractChannel,
};
use serde::Serialize;
use serde_json;
use std::{
    string::ToString,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

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
    moz_prover.base_extend(channel).unwrap();
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
    moz_verifier.base_extend(channel).unwrap();
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
        // RingParameter::R72 => run_benchmark::<z2r::R72>(&options),
        // RingParameter::R78 => run_benchmark::<z2r::R78>(&options),
        RingParameter::R104 => run_benchmark::<z2r::R104>(&options),
        // RingParameter::R110 => run_benchmark::<z2r::R110>(&options),
        // RingParameter::R112 => run_benchmark::<z2r::R112>(&options),
        // RingParameter::R118 => run_benchmark::<z2r::R118>(&options),
        // RingParameter::R119 => run_benchmark::<z2r::R119>(&options),
        RingParameter::R144 => run_benchmark::<z2r::R144>(&options),
        RingParameter::R150 => run_benchmark::<z2r::R150>(&options),
        // RingParameter::R151 => run_benchmark::<z2r::R151>(&options),
        // RingParameter::R196 => run_benchmark::<z2r::R196>(&options),
        // RingParameter::R203 => run_benchmark::<z2r::R203>(&options),
        RingParameter::R224 => run_benchmark::<z2r::R224>(&options),
        RingParameter::R231 => run_benchmark::<z2r::R231>(&options),
        RingParameter::RX => run_benchmark::<RX>(&options),
        _ => println!("selected ring {} not compiled in", options.ring.to_string()),
    }
}

fn main() {
    run()
}
