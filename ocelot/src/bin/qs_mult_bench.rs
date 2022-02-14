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
        MozzarellaProver, MozzarellaVerifier,
    },
    quicksilver::{
        QuicksilverProver, QuicksilverProverStats, QuicksilverVerifier, QuicksilverVerifierStats,
    },
    tools::BenchmarkMetaData,
};
use rand::distributions::{Distribution, Standard};
use rayon;
use scuttlebutt::{
    channel::{track_unix_channel_pair, Receivable, Sendable},
    ring::{z2r, Ring, R64},
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
    name = "QuarkSilver Multiplication Benchmark",
    author = "Alex Hansen, Lennart Braun",
    version = "0.1"
)]
struct Options {
    #[clap(short = 'P', long, arg_enum)]
    party: Party,

    #[clap(short = 'R', long, arg_enum, default_value_t = RingParameter::R64)]
    ring: RingParameter,

    #[clap(short = 'k', long)]
    plain_size: usize,

    #[clap(short, long)]
    statsec: usize,

    #[clap(flatten, help_heading = "LPN parameters")]
    lpn_parameters: LpnParameters,

    #[clap(
        short,
        long,
        default_value_t = 1,
        help_heading = "Number of multiplicaitons to verifiy"
    )]
    num_mults: usize,

    #[clap(flatten, help_heading = "network options")]
    network_options: NetworkOptions,

    #[clap(short, long, default_value_t = 0)]
    threads: usize,

    #[clap(short, long, default_value_t = 1)]
    repetitions: usize,

    #[clap(long)]
    nightly: bool,

    #[clap(short, long)]
    json: bool,

    #[clap(short, long)]
    verbose: bool,
}

#[derive(Clone, Debug, Serialize)]
enum PartyStats {
    ProverStats(QuicksilverProverStats),
    VerifierStats(QuicksilverVerifierStats),
}

#[derive(Clone, Debug, Default, Serialize)]
struct StatTuple {
    pub n: usize,
    pub ns_avg: f64,
    pub ns_avg_per_mult: f64,
    pub ns_median: f64,
    pub ns_median_per_mult: f64,
    pub ns_stddev: f64,
    pub ns_stddev_per_mult: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
struct RunTimeStats {
    pub init_run_times: Vec<Duration>,
    pub mult_voles_run_times: Vec<Duration>,
    pub mults_run_times: Vec<Duration>,
    pub check_run_times: Vec<Duration>,
    pub init_stats: StatTuple,
    pub mult_voles_stats: StatTuple,
    pub mults_stats: StatTuple,
    pub check_stats: StatTuple,
    pub kilobytes_sent: f64,
    pub kilobytes_received: f64,
    pub party_stats: Vec<PartyStats>,
}

impl RunTimeStats {
    fn analyse_times(times: &[Duration], num_mults: usize) -> StatTuple {
        let n = times.len();
        assert!(n > 0);
        let mut ns: Vec<u128> = times.iter().map(|d| d.as_nanos()).collect();
        ns.sort_unstable();
        let ns_avg = ns.iter().sum::<u128>() as f64 / n as f64;
        let ns_median = ns[n / 2] as f64;
        let ns_stddev = if n > 1 {
            (ns.iter()
                .map(|x| (*x as f64 - ns_avg).powf(2f64))
                .sum::<f64>()
                / (n - 1) as f64)
                .sqrt()
        } else {
            f64::NAN
        };
        StatTuple {
            n,
            ns_avg,
            ns_avg_per_mult: ns_avg / num_mults as f64,
            ns_median,
            ns_median_per_mult: ns_median / num_mults as f64,
            ns_stddev,
            ns_stddev_per_mult: ns_stddev / num_mults as f64,
        }
    }

    pub fn compute_statistics(&mut self, num_mults: usize) {
        self.init_stats = Self::analyse_times(&self.init_run_times, num_mults);
        self.mult_voles_stats = Self::analyse_times(&self.mult_voles_run_times, num_mults);
        self.mults_stats = Self::analyse_times(&self.mults_run_times, num_mults);
        self.check_stats = Self::analyse_times(&self.check_run_times, num_mults);
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
                mults_run_times: Vec::with_capacity(options.repetitions),
                mult_voles_run_times: Vec::with_capacity(options.repetitions),
                check_run_times: Vec::with_capacity(options.repetitions),
                init_stats: Default::default(),
                mult_voles_stats: Default::default(),
                mults_stats: Default::default(),
                check_stats: Default::default(),
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
    plain_size: usize,
    statsec: usize,
    lpn_parameters: LpnParameters,
    code: &LLCode<RingT>,
    cache: CachedProver<RingT>,
    num_mults: usize,
    _nightly: bool,
) -> (Duration, Duration, Duration, Duration, PartyStats)
where
    RingT: Ring + Receivable,
    for<'b> &'b RingT: Sendable,
    Standard: Distribution<RingT>,
{
    let mut qs_prover = QuicksilverProver::<RingT>::new(
        plain_size,
        statsec,
        cache,
        code,
        lpn_parameters.base_vole_size,
        lpn_parameters.num_noise_coordinates,
        lpn_parameters.get_block_size(),
    );

    let t_start = Instant::now();
    qs_prover.init(channel).unwrap();
    let run_time_init = t_start.elapsed();

    let _: bool = channel.receive().unwrap();
    channel.send(true).unwrap();

    // prepare inputs
    let (alphas, alpha_macs) = qs_prover
        .random_batch(channel, num_mults)
        .expect("random failed");
    let (betas, beta_macs) = qs_prover
        .random_batch(channel, num_mults)
        .expect("random failed");

    qs_prover.apply_to_mozzarella_prover(|p| p.drain_cache());

    // compute multiplications
    let t_start = Instant::now();
    qs_prover
        .apply_to_mozzarella_prover(|p: &mut MozzarellaProver<RingT>| {
            p.ensure(channel, num_mults + 1)
        })
        .unwrap();
    let run_time_mult_voles = t_start.elapsed();

    let t_start = Instant::now();
    let (gammas, gamma_macs) = qs_prover
        .multiply_batch(channel, (&alphas, &alpha_macs), (&betas, &beta_macs))
        .expect("multiply failed");
    let run_time_mults = t_start.elapsed();

    let _: bool = channel.receive().unwrap();
    channel.send(true).unwrap();

    let t_start = Instant::now();
    qs_prover
        .check_multiply_batch(
            channel,
            (&alphas, &alpha_macs),
            (&betas, &beta_macs),
            (&gammas, &gamma_macs),
        )
        .expect("check_multiply failed");
    let run_time_check = t_start.elapsed();

    (
        run_time_init,
        run_time_mult_voles,
        run_time_mults,
        run_time_check,
        PartyStats::ProverStats(qs_prover.get_stats()),
    )
}

fn run_verifier<RingT, C: AbstractChannel>(
    channel: &mut C,
    plain_size: usize,
    statsec: usize,
    lpn_parameters: LpnParameters,
    code: &LLCode<RingT>,
    cache: CachedVerifier<RingT>,
    delta: RingT,
    num_mults: usize,
    _nightly: bool,
) -> (Duration, Duration, Duration, Duration, PartyStats)
where
    RingT: Ring + Receivable,
    for<'b> &'b RingT: Sendable,
    Standard: Distribution<RingT>,
{
    let mut qs_verifier = QuicksilverVerifier::<RingT>::new(
        plain_size,
        statsec,
        cache,
        code,
        lpn_parameters.base_vole_size,
        lpn_parameters.num_noise_coordinates,
        lpn_parameters.get_block_size(),
    );
    let t_start = Instant::now();
    qs_verifier.init(channel, delta).unwrap();
    let run_time_init = t_start.elapsed();

    channel.send(true).unwrap();
    let _: bool = channel.receive().unwrap();

    // prepare inputs
    let alpha_keys = qs_verifier
        .random_batch(channel, num_mults)
        .expect("random failed");
    let beta_keys = qs_verifier
        .random_batch(channel, num_mults)
        .expect("random failed");

    qs_verifier.apply_to_mozzarella_verifier(|p: &mut MozzarellaVerifier<RingT>| p.drain_cache());

    // compute multiplications
    let t_start = Instant::now();
    qs_verifier
        .apply_to_mozzarella_verifier(|p: &mut MozzarellaVerifier<RingT>| {
            p.ensure(channel, num_mults + 1)
        })
        .unwrap();
    let run_time_mult_voles = t_start.elapsed();

    let t_start = Instant::now();
    let gamma_keys = qs_verifier
        .multiply_batch(channel, &alpha_keys, &beta_keys)
        .expect("multiply failed");
    let run_time_mults = t_start.elapsed();

    channel.send(true).unwrap();
    let _: bool = channel.receive().unwrap();

    let t_start = Instant::now();
    qs_verifier
        .check_multiply_batch(channel, &alpha_keys, &beta_keys, &gamma_keys)
        .expect("check_multiply failed");
    let run_time_check = t_start.elapsed();

    (
        run_time_init,
        run_time_mult_voles,
        run_time_mults,
        run_time_check,
        PartyStats::VerifierStats(qs_verifier.get_stats()),
    )
}

fn run_benchmark<RingT>(options: &Options)
where
    RingT: Ring + Receivable,
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
            let plain_size = options.plain_size;
            let statsec = options.statsec;
            let lpn_parameters_p = options.lpn_parameters;
            let lpn_parameters_v = options.lpn_parameters;
            let code_p = Arc::new(code);
            let code_v = code_p.clone();
            let repetitions = options.repetitions;
            let num_mults = options.num_mults;
            let nightly = options.nightly;
            let mut results_p = BenchmarkResult::new(&options);
            let mut results_v = results_p.clone();
            let prover_thread = thread::spawn(move || {
                for _ in 0..repetitions {
                    let (
                        run_time_init,
                        run_time_mult_voles,
                        run_time_mults,
                        run_time_check,
                        party_stats,
                    ) = run_prover::<RingT, _>(
                        &mut channel_p,
                        plain_size,
                        statsec,
                        lpn_parameters_p,
                        &code_p,
                        prover_cache.clone(),
                        num_mults,
                        nightly,
                    );
                    results_p.run_time_stats.init_run_times.push(run_time_init);
                    results_p
                        .run_time_stats
                        .mult_voles_run_times
                        .push(run_time_mult_voles);
                    results_p
                        .run_time_stats
                        .mults_run_times
                        .push(run_time_mults);
                    results_p
                        .run_time_stats
                        .check_run_times
                        .push(run_time_check);
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
                    let (
                        run_time_init,
                        run_time_mult_voles,
                        run_time_mults,
                        run_time_check,
                        party_stats,
                    ) = run_verifier::<RingT, _>(
                        &mut channel_v,
                        plain_size,
                        statsec,
                        lpn_parameters_v,
                        &code_v,
                        verifier_cache.clone(),
                        delta,
                        num_mults,
                        nightly,
                    );
                    results_v.run_time_stats.init_run_times.push(run_time_init);
                    results_v
                        .run_time_stats
                        .mult_voles_run_times
                        .push(run_time_mult_voles);
                    results_v
                        .run_time_stats
                        .mults_run_times
                        .push(run_time_mults);
                    results_v
                        .run_time_stats
                        .check_run_times
                        .push(run_time_check);
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
                .compute_statistics(options.num_mults);
            let mut results_v = verifier_thread.join().unwrap();
            results_v
                .run_time_stats
                .compute_statistics(options.num_mults);
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
                let (
                    run_time_init,
                    run_time_mult_voles,
                    run_time_mults,
                    run_time_check,
                    party_stats,
                ) = match party {
                    Party::Prover => run_prover::<RingT, _>(
                        &mut channel,
                        options.plain_size,
                        options.statsec,
                        options.lpn_parameters,
                        &code,
                        prover_cache.clone(),
                        options.num_mults,
                        options.nightly,
                    ),
                    Party::Verifier => run_verifier::<RingT, _>(
                        &mut channel,
                        options.plain_size,
                        options.statsec,
                        options.lpn_parameters,
                        &code,
                        verifier_cache.clone(),
                        delta,
                        options.num_mults,
                        options.nightly,
                    ),
                    _ => panic!("can't happen"),
                };
                results.run_time_stats.init_run_times.push(run_time_init);
                results
                    .run_time_stats
                    .mult_voles_run_times
                    .push(run_time_mult_voles);
                results.run_time_stats.mults_run_times.push(run_time_mults);
                results.run_time_stats.check_run_times.push(run_time_check);
                results.run_time_stats.party_stats.push(party_stats);
                results.repetitions += 1;
                if results.repetitions == 1 {
                    results.run_time_stats.kilobytes_sent = channel.kilobytes_written();
                    results.run_time_stats.kilobytes_received = channel.kilobytes_read();
                }
                if !options.json {
                    println!("{:?} time (init): {:?}", options.party, run_time_init);
                    println!(
                        "{:?} time (mult_voles): {:?}",
                        options.party, run_time_mult_voles
                    );
                    println!("{:?} time (mults): {:?}", options.party, run_time_mults);
                    println!("{:?} time (check): {:?}", options.party, run_time_check);
                    println!("sent data: {:.2} MiB", channel.kilobytes_written() / 1024.0);
                    println!(
                        "received data: {:.2} MiB",
                        channel.kilobytes_read() / 1024.0
                    );
                }
            }
            results.run_time_stats.compute_statistics(options.num_mults);
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
        RingParameter::R130 => run_benchmark::<z2r::R130>(&options),
        RingParameter::R162 => run_benchmark::<z2r::R162>(&options),
        RingParameter::R212 => run_benchmark::<z2r::R212>(&options),
        RingParameter::R244 => run_benchmark::<z2r::R244>(&options),
        _ => println!("selected ring {} not compiled in", options.ring.to_string()),
    }
}

fn main() {
    run()
}
