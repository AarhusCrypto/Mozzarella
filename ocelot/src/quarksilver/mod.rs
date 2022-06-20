use crate::quarksilver::prover::{Prover, ProverStats};
use crate::quarksilver::verifier::{Verifier, VerifierStats};

mod prover;
mod verifier;

pub type QuarkSilverProver<'a, RingT> = Prover<'a, RingT>;
pub type QuarkSilverVerifier<'a, RingT> = Verifier<'a, RingT>;

pub type QuarkSilverProverStats = ProverStats;
pub type QuarkSilverVerifierStats = VerifierStats;
