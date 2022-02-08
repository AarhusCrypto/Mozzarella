use crate::{
    ot::mozzarella::{
        cache::{cacheinit::GenCache, prover::CachedProver, verifier::CachedVerifier},
        lpn::LLCode,
        CODE_D,
    },
    Error,
};
use clap;
use rand::{
    distributions::{Distribution, Standard},
    Rng, SeedableRng,
};
use scuttlebutt::{ring::Ring, AesRng, Block};
use scuttlebutt::{SyncChannel, TrackChannel};
use serde::Serialize;
use std::{
    fmt,
    io::{BufReader, BufWriter},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

#[derive(Debug, Clone, clap::ArgEnum)]
pub enum Party {
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

#[derive(Debug, Clone, clap::Parser, Serialize)]
pub struct NetworkOptions {
    #[clap(short, long)]
    listen: bool,
    #[clap(short, long, default_value = "localhost")]
    host: String,
    #[clap(short, long, default_value_t = 1337)]
    port: u16,
    #[clap(long, default_value_t = 100)]
    connect_timeout_seconds: usize,
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

pub fn setup_network(options: &NetworkOptions) -> Result<NetworkChannel, Error> {
    if options.listen {
        listen(&options.host, options.port)
    } else {
        connect(&options.host, options.port, options.connect_timeout_seconds)
    }
}

#[derive(Debug, Clone, clap::ArgEnum)]
pub enum RingParameter {
    R64,
    R72,
    R78,
    R104,
    R110,
    R112,
    R118,
    R119,
    R144,
    R150,
    R151,
    R196,
    R203,
    R224,
    R231,
}

impl fmt::Display for RingParameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            RingParameter::R64 => write!(f, "R64"),
            RingParameter::R72 => write!(f, "R72"),
            RingParameter::R78 => write!(f, "R72"),
            RingParameter::R104 => write!(f, "R104"),
            RingParameter::R110 => write!(f, "R110"),
            RingParameter::R112 => write!(f, "R112"),
            RingParameter::R118 => write!(f, "R118"),
            RingParameter::R119 => write!(f, "R119"),
            RingParameter::R144 => write!(f, "R144"),
            RingParameter::R150 => write!(f, "R150"),
            RingParameter::R151 => write!(f, "R151"),
            RingParameter::R196 => write!(f, "R196"),
            RingParameter::R203 => write!(f, "R203"),
            RingParameter::R224 => write!(f, "R224"),
            RingParameter::R231 => write!(f, "R231"),
        }
    }
}

#[derive(Debug, Copy, Clone, clap::Parser, Serialize)]
pub struct LpnParameters {
    #[clap(short = 'K', long)]
    pub base_vole_size: usize,
    #[clap(short = 'N', long)]
    pub extension_size: usize,
    #[clap(short = 'T', long)]
    pub num_noise_coordinates: usize,
}

impl LpnParameters {
    pub fn recompute_extension_size(&mut self) {
        assert!(self.num_noise_coordinates > 0);
        // increase extension_size s.t. it is a multiple of the number of noise coordinates
        // ceil(extension_size / num_noise_coordinates)
        let block_size = 1 + (self.extension_size - 1) / self.num_noise_coordinates;
        // recompute extension size to be a multiple of the block size
        self.extension_size = block_size * self.num_noise_coordinates;
    }

    pub fn get_block_size(&self) -> usize {
        1 + (self.extension_size - 1) / self.num_noise_coordinates
    }

    pub fn get_required_cache_size(&self) -> usize {
        self.base_vole_size + 2 * self.num_noise_coordinates
    }

    pub fn get_vole_output_size(&self) -> usize {
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

pub fn setup_cache<RingT>(
    lpn_parameters: &LpnParameters,
) -> (CachedProver<RingT>, (CachedVerifier<RingT>, RingT))
where
    RingT: Ring,
    Standard: Distribution<RingT>,
{
    let mut rng = AesRng::from_seed(Default::default());
    let delta = rng.gen::<RingT>();
    let (prover_cache, verifier_cache) =
        GenCache::new_with_size(rng, delta, lpn_parameters.get_required_cache_size());
    (prover_cache, (verifier_cache, delta))
}

pub fn generate_code<RingT>(lpn_parameters: &LpnParameters) -> LLCode<RingT>
where
    RingT: Ring,
    Standard: Distribution<RingT>,
{
    LLCode::<RingT>::from_seed(
        lpn_parameters.base_vole_size,
        lpn_parameters.extension_size,
        CODE_D,
        Block::default(),
    )
}
