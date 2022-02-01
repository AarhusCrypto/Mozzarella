use crate::quicksilver::prover::Prover;
use crate::quicksilver::verifier::Verifier;

mod prover;
mod verifier;

pub type QuicksilverProver<'a, RingT> = Prover<'a, RingT>;
pub type QuicksilverVerifier<'a, RingT> = Verifier<'a, RingT>;