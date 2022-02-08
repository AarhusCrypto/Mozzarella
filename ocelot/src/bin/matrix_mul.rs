use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use clap::{ErrorKind, IntoApp, Parser};
use rand::distributions::{Distribution, Standard};
use rand::{CryptoRng, Rng, SeedableRng};
use serde::Serialize;
use serde_json;

use ocelot::benchmark_tools::{
    generate_code, setup_cache, setup_network, LpnParameters, NetworkOptions, Party, RingParameter,
};
use ocelot::ot::mozzarella::cache::prover::CachedProver;
use ocelot::ot::mozzarella::cache::verifier::CachedVerifier;
use ocelot::ot::mozzarella::lpn::LLCode;
use ocelot::quicksilver::{
    QuicksilverProver, QuicksilverProverStats, QuicksilverVerifier, QuicksilverVerifierStats,
};
use ocelot::tools::BenchmarkMetaData;

use scuttlebutt::channel::{Receivable, Sendable};
use scuttlebutt::ring::{NewRing, R64};
use scuttlebutt::{track_unix_channel_pair, AbstractChannel, AesRng, Block};

const CHUNK_SIZE: usize = 10000;

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

    // what the hell does "long" mean?
    #[clap(short = 'D', long)]
    dim: usize,

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

    #[clap(short = 'M', long)]
    multi_thread: bool,
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
    pub multiplication_check_time: Vec<Duration>,
    pub init_stats: StatTuple,
    pub extend_stats: StatTuple,
    pub mul_check_stats: StatTuple,
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
        //self.extend_stats = Self::analyse_times(&self.extend_run_times, num_voles);
    }

    pub fn compute_quicksilver_statistics(&mut self, num_muls: usize) {
        self.mul_check_stats = Self::analyse_times(&self.multiplication_check_time, num_muls);
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
                multiplication_check_time: Vec::with_capacity(options.repetitions),
                init_stats: Default::default(),
                extend_stats: Default::default(),
                mul_check_stats: Default::default(),
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

#[allow(non_snake_case)]
fn generate_matrices<RingT, R: CryptoRng + Rng>(
    delta: RingT,
    mut rng: R,
    dim: usize,
) -> (
    Vec<(RingT, RingT)>,
    Vec<(RingT, RingT)>,
    Vec<RingT>,
    Vec<RingT>,
    Vec<RingT>,
)
where
    RingT: NewRing,
    Standard: Distribution<RingT>,
{
    let mut prover_A: Vec<(RingT, RingT)> = vec![(RingT::default(), RingT::default()); dim.pow(2)];
    let mut prover_B: Vec<(RingT, RingT)> = vec![(RingT::default(), RingT::default()); dim.pow(2)];
    let mut C: Vec<RingT> = vec![RingT::default(); dim.pow(2)];

    let mut verifier_A: Vec<RingT> = vec![RingT::default(); dim.pow(2)];
    let mut verifier_B: Vec<RingT> = vec![RingT::default(); dim.pow(2)];

    for i in 0..dim {
        for j in 0..dim {
            let a1: RingT = rng.gen();
            let b1: RingT = rng.gen();
            let a2: RingT = a1 * delta + b1;

            let a3: RingT = rng.gen();
            let b2: RingT = rng.gen();
            let a4: RingT = a3 * delta + b2;

            prover_A[i * dim + j] = (a1, a2);
            prover_B[i * dim + j] = (a3, a4);

            verifier_A[i * dim + j] = b1;
            verifier_B[i * dim + j] = b2;

            C[i * dim + j] = a1 * a3;
        }
    }

    (prover_A, prover_B, verifier_A, verifier_B, C)
}

#[allow(non_snake_case)]
fn run_verifier<RingT, C: AbstractChannel>(
    channel: &mut C,
    lpn_parameters: LpnParameters,
    code: &LLCode<RingT>,
    cache: CachedVerifier<RingT>,
    mut delta: RingT,
    dim: usize,
    verifier_A: Vec<RingT>,
    verifier_B: Vec<RingT>,
    _C_mat: &Vec<RingT>,
    multi_thread: bool,
    _nightly: bool,
) -> (Duration, Duration, PartyStats)
where
    RingT: NewRing + Receivable,
    for<'b> &'b RingT: Sendable,
    Standard: Distribution<RingT>,
{
    let rng = AesRng::from_seed(Block::default());

    let mut quicksilver_verifier = QuicksilverVerifier::<RingT>::new(
        cache,
        code,
        lpn_parameters.base_vole_size,
        lpn_parameters.num_noise_coordinates,
        lpn_parameters.get_block_size(),
    );

    let init_start = Instant::now();
    quicksilver_verifier.init(channel, delta);
    let init_time = init_start.elapsed();

    let mut triples: Vec<(RingT, RingT, RingT)> = Vec::new();

    for row in 0..dim {
        for col in 0..dim {
            for i in 0..dim {
                let out = quicksilver_verifier
                    .multiply(
                        channel,
                        (verifier_A[row * dim + i], verifier_B[i * dim + col]),
                    )
                    .unwrap();

                //C[row][col] = out.2;
                triples.push(out);
            }
        }
    }

    let t_start = Instant::now();
    quicksilver_verifier
        .check_multiply(
            channel,
            rng,
            triples.as_mut_slice(),
            multi_thread,
            CHUNK_SIZE,
        )
        .unwrap();
    let run_time_multiply = t_start.elapsed();
    let stats = quicksilver_verifier.get_stats();

    (
        init_time,
        run_time_multiply,
        PartyStats::VerifierStats(stats),
    )
}

#[allow(non_snake_case)]
fn run_prover<RingT, C: AbstractChannel>(
    channel: &mut C,
    lpn_parameters: LpnParameters,
    code: &LLCode<RingT>,
    cache: CachedProver<RingT>,
    dim: usize,
    prover_A: Vec<(RingT, RingT)>,
    prover_B: Vec<(RingT, RingT)>,
    _C_mat: &Vec<RingT>,
    multi_thread: bool,
    _nightly: bool,
) -> (Duration, Duration, PartyStats)
where
    RingT: NewRing + Receivable,
    for<'b> &'b RingT: Sendable,
    Standard: Distribution<RingT>,
{
    let mut quicksilver_prover = QuicksilverProver::<RingT>::new(
        cache,
        code,
        lpn_parameters.base_vole_size,
        lpn_parameters.num_noise_coordinates,
        lpn_parameters.get_block_size(),
    );

    let init_start = Instant::now();
    quicksilver_prover.init(channel);
    let init_time = init_start.elapsed();

    let mut triples: Vec<((RingT, RingT), (RingT, RingT), (RingT, RingT))> = Vec::new();

    for row in 0..dim {
        for col in 0..dim {
            //let mut tmp: RingT = RingT::default();
            for i in 0..dim {
                let out = quicksilver_prover
                    .multiply(channel, prover_A[row * dim + i], prover_B[i * dim + col])
                    .unwrap();
                //tmp += out.2.0;
                triples.push(out);
            }
            //C[row][col] = tmp;
        }
    }

    let t_start = Instant::now();
    quicksilver_prover
        .check_multiply(channel, triples.as_mut_slice(), multi_thread, CHUNK_SIZE)
        .unwrap();
    let run_time_multiply = t_start.elapsed();
    let stats = quicksilver_prover.get_stats();

    (init_time, run_time_multiply, PartyStats::ProverStats(stats))
}

#[allow(non_snake_case)]
fn run_matrix_mul_benchmark<RingT>(options: &Options)
where
    RingT: NewRing + Receivable,
    for<'b> &'b RingT: Sendable,
    Standard: Distribution<RingT>,
{
    rayon::ThreadPoolBuilder::new()
        .num_threads(0)
        .build_global()
        .unwrap();

    let mut results = BenchmarkResult::new(&options);
    let (prover_cache, (verifier_cache, delta)) = setup_cache(&options.lpn_parameters);
    let code = generate_code::<RingT>(&options.lpn_parameters);

    // todo: should we generate these fresh each iteration to make sure the cpu won't do any tricks?
    let (prover_A, prover_B, verifier_A, verifier_B, C) =
        generate_matrices(delta, AesRng::from_seed(Block::default()), options.dim);

    match &options.party {
        Party::Both => {
            let (mut channel_p, mut channel_v) = track_unix_channel_pair();
            let lpn_parameters_p = options.lpn_parameters;
            let lpn_parameters_v = options.lpn_parameters;

            let repetitions = options.repetitions;
            let dim = options.dim;
            let code_p = Arc::new(code);
            let code_v = code_p.clone();

            let mt_p = options.multi_thread;
            let mt_v = options.multi_thread;

            let C_p = Arc::new(C);
            let C_v = C_p.clone();

            let nightly = options.nightly;

            let mut results_p = BenchmarkResult::new(&options);
            let mut results_v = results_p.clone();

            let prover_thread: JoinHandle<BenchmarkResult> = thread::spawn(move || {
                for _ in 0..repetitions {
                    let (run_time_init, run_time_multiply, party_stats) = run_prover::<RingT, _>(
                        &mut channel_p,
                        lpn_parameters_p,
                        &code_p,
                        prover_cache.clone(),
                        dim,
                        prover_A.clone(),
                        prover_B.clone(),
                        &C_p,
                        mt_p,
                        nightly,
                    );

                    results_p.run_time_stats.init_run_times.push(run_time_init);
                    results_p
                        .run_time_stats
                        .multiplication_check_time
                        .push(run_time_multiply);
                    results_p.run_time_stats.party_stats.push(party_stats);
                    results_p.repetitions += 1;
                    if results_p.repetitions == 1 {
                        // only need this once, it's static lol
                        results_p.run_time_stats.kilobytes_sent = channel_p.kilobytes_written();
                        results_p.run_time_stats.kilobytes_received = channel_p.kilobytes_read();
                    }
                }
                results_p
            });

            let verifier_thread: JoinHandle<BenchmarkResult> = thread::spawn(move || {
                for _ in 0..repetitions {
                    let (run_time_init, run_time_multiply, party_stats) = run_verifier::<RingT, _>(
                        &mut channel_v,
                        lpn_parameters_v,
                        &code_v,
                        verifier_cache.clone(),
                        delta,
                        dim,
                        verifier_A.clone(),
                        verifier_B.clone(),
                        &C_v,
                        mt_v,
                        nightly,
                    );

                    results_v.run_time_stats.init_run_times.push(run_time_init);
                    results_v
                        .run_time_stats
                        .multiplication_check_time
                        .push(run_time_multiply);
                    results_v.run_time_stats.party_stats.push(party_stats);
                    results_v.repetitions += 1;
                    if results_v.repetitions == 1 {
                        // only need this once, it's static lol
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

            results_p
                .run_time_stats
                .compute_quicksilver_statistics(dim.pow(3));

            let mut results_v = verifier_thread.join().unwrap();
            results_v
                .run_time_stats
                .compute_statistics(options.lpn_parameters.get_vole_output_size());
            results_v
                .run_time_stats
                .compute_quicksilver_statistics(dim.pow(3));

            if options.json {
                println!("{}", serde_json::to_string_pretty(&results_p).unwrap());
                println!("{}", serde_json::to_string_pretty(&results_v).unwrap());
            } else {
                println!("results prover: {:?}", results_p);
                println!("results verifier: {:?}", results_v);
            }
        }
        party => {
            println!("Setting up channel");
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
                println!("Running");
                let (run_time_init, run_time_multiply, party_stats) = match party {
                    Party::Prover => run_prover::<RingT, _>(
                        &mut channel,
                        options.lpn_parameters,
                        &code,
                        prover_cache.clone(),
                        options.dim,
                        prover_A.clone(),
                        prover_B.clone(),
                        &C,
                        options.multi_thread,
                        options.nightly,
                    ),
                    Party::Verifier => run_verifier::<RingT, _>(
                        &mut channel,
                        options.lpn_parameters,
                        &code,
                        verifier_cache.clone(),
                        delta,
                        options.dim,
                        verifier_A.clone(),
                        verifier_B.clone(),
                        &C,
                        options.multi_thread,
                        options.nightly,
                    ),
                    _ => panic!("can't happen"),
                };

                results.run_time_stats.init_run_times.push(run_time_init);
                results
                    .run_time_stats
                    .multiplication_check_time
                    .push(run_time_multiply);
                results.run_time_stats.party_stats.push(party_stats);
                results.repetitions += 1;
                if results.repetitions == 1 {
                    results.run_time_stats.kilobytes_sent = channel.kilobytes_written();
                    results.run_time_stats.kilobytes_received = channel.kilobytes_read();
                }
                if !options.json {
                    println!("{:?} time (init): {:?}", options.party, run_time_init);
                    println!("{:?} time (vole): {:?}", options.party, run_time_multiply);
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

            results
                .run_time_stats
                .compute_quicksilver_statistics(options.dim.pow(3));
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
    /*
       match options.ring {
           RingParameter::R64 => run_matrix_mul_benchmark::<R64>(&options),
           // RingParameter::R72 => run_benchmark::<z2r::R72>(&options),
           // RingParameter::R78 => run_benchmark::<z2r::R78>(&options),
           RingParameter::R104 => run_matrix_mul_benchmark::<z2r::R104>(&options),
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
    */
}

fn main() {
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

    run_matrix_mul_benchmark::<R64>(&options)
}
