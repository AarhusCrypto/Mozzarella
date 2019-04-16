// -*- mode: rust; -*-
//
// This file is part of ocelot.
// Copyright © 2019 Galois, Inc.
// See LICENSE for licensing information.

//! Implementation of the Chou-Orlandi oblivious transfer protocol (cf.
//! <https://eprint.iacr.org/2015/267>).
//!
//! This implementation uses the Ristretto prime order elliptic curve group from
//! the `curve25519-dalek` library and works over blocks rather than arbitrary
//! length messages.
//!
//! This version fixes a bug in the current ePrint write-up
//! (<https://eprint.iacr.org/2015/267/20180529:135402>, Page 4): if the value
//! `x^i` produced by the receiver is not randomized, all the random-OTs
//! produced by the protocol will be the same. We fix this by hashing in `i`
//! during the key derivation phase.

use crate::errors::Error;
use crate::ot::{Receiver as OtReceiver, Sender as OtSender};
use crate::stream;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_TABLE;
use curve25519_dalek::ristretto::{RistrettoBasepointTable, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;
use rand::{CryptoRng, RngCore};
use scuttlebutt::{Block, Malicious, SemiHonest};
use std::io::{Read, Write};

/// Oblivious transfer sender.
pub struct Sender {
    y: Scalar,
    s: RistrettoPoint,
}

impl OtSender for Sender {
    type Msg = Block;

    fn init<R: Read, W: Write, RNG: CryptoRng + RngCore>(
        _: &mut R,
        writer: &mut W,
        mut rng: &mut RNG,
    ) -> Result<Self, Error> {
        let y = Scalar::random(&mut rng);
        let s = &y * &RISTRETTO_BASEPOINT_TABLE;
        stream::write_pt(writer, &s)?;
        writer.flush()?;
        Ok(Self { y, s })
    }

    fn send<R: Read, W: Write, RNG: CryptoRng + RngCore>(
        &mut self,
        reader: &mut R,
        writer: &mut W,
        inputs: &[(Block, Block)],
        _: &mut RNG,
    ) -> Result<(), Error> {
        let ks = (0..inputs.len())
            .map(|i| {
                let r = stream::read_pt(reader)?;
                let k0 = Block::hash_pt(i, &(self.y * r));
                let k1 = Block::hash_pt(i, &(self.y * (r - self.s)));
                Ok((k0, k1))
            })
            .collect::<Result<Vec<(Block, Block)>, Error>>()?;
        for (input, k) in inputs.iter().zip(ks.into_iter()) {
            let c0 = k.0 ^ input.0;
            let c1 = k.1 ^ input.1;
            c0.write(writer)?;
            c1.write(writer)?;
        }
        writer.flush()?;
        Ok(())
    }
}

impl std::fmt::Display for Sender {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Chou-Orlandi Sender")
    }
}

/// Oblivious transfer receiver.
pub struct Receiver {
    s: RistrettoBasepointTable,
}

impl OtReceiver for Receiver {
    type Msg = Block;

    fn init<R: Read, W: Write, RNG: CryptoRng + RngCore>(
        reader: &mut R,
        _: &mut W,
        _: &mut RNG,
    ) -> Result<Self, Error> {
        let s = stream::read_pt(reader)?;
        let s = RistrettoBasepointTable::create(&s);
        Ok(Self { s })
    }

    fn receive<R: Read, W: Write, RNG: CryptoRng + RngCore>(
        &mut self,
        reader: &mut R,
        writer: &mut W,
        inputs: &[bool],
        mut rng: &mut RNG,
    ) -> Result<Vec<Block>, Error> {
        let zero = &Scalar::zero() * &self.s;
        let one = &Scalar::one() * &self.s;
        let ks = inputs
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let x = Scalar::random(&mut rng);
                let c = if *b { one } else { zero };
                let r = c + &x * &RISTRETTO_BASEPOINT_TABLE;
                stream::write_pt(writer, &r)?;
                Ok(Block::hash_pt(i, &(&x * &self.s)))
            })
            .collect::<Result<Vec<Block>, Error>>()?;
        writer.flush()?;
        inputs
            .iter()
            .zip(ks.into_iter())
            .map(|(b, k)| {
                let c0 = Block::read(reader)?;
                let c1 = Block::read(reader)?;
                let c = k ^ if *b { c1 } else { c0 };
                Ok(c)
            })
            .collect()
    }
}

impl std::fmt::Display for Receiver {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Chou-Orlandi Receiver")
    }
}

impl SemiHonest for Sender {}
impl Malicious for Sender {}
impl SemiHonest for Receiver {}
impl Malicious for Receiver {}
