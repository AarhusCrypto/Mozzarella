use crate::quicksilver::prover::{Prover, ProverStats};
use crate::quicksilver::verifier::{Verifier, VerifierStats};

mod prover;
mod verifier;

pub type QuicksilverProver<'a, RingT> = Prover<'a, RingT>;
pub type QuicksilverVerifier<'a, RingT> = Verifier<'a, RingT>;

pub type QuicksilverProverStats = ProverStats;
pub type QuicksilverVerifierStats = VerifierStats;
