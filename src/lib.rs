// -*- mode: rust; -*-
//
// This file is part of `scuttlebutt`.
// Copyright © 2019 Galois, Inc.
// See LICENSE for licensing information.

//! `scuttlebutt` is a library of primitives for use in multi-party computation
//! protocols.
//!
//! `scuttlebutt` provides the following:
//! * `AesHash`, which provides a correlation-robust hash function based on
//! fixed-key AES (cf. <https://eprint.iacr.org/2019/074>).
//! * `AesRng`, which provides a random number generator based on fixed-key AES.
//! * `Block`, which wraps an `__m128i` type and provides methods useful
//! when used as a garbled circuit wire label.

#![allow(clippy::many_single_char_names)]
#![cfg_attr(feature = "nightly", feature(stdsimd))]
#![cfg_attr(feature = "nightly", feature(test))]

mod aes;
mod block;
mod hash_aes;
mod rand_aes;

pub use crate::block::Block;
pub use crate::hash_aes::AesHash;
pub use crate::rand_aes::AesRng;
