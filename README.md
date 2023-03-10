# Mozzarella Benchmarking Code

This repository contains the code developed for the benchmarking experiments in our paper:

"Moz​$\mathbb{Z}\_{2^k}$​arella: Efficient Vector-OLE and Zero-Knowledge Proofs Over $\mathbb{Z}\_{2^k}$". By *Carsten Baum, Lennart Braun, Alexander Munch-Hansen, and Peter Scholl* (all Aarhus University). [Crypto 2022](https://crypto.iacr.org/2022/). [Full version on ePrint](https://eprint.iacr.org/2022/819).


## Code

The code is based on [the secure computation framework *swanky* by Galois](https://github.com/GaloisInc/swanky), more specifically, on a [fork of swanky by Mathias Hall-Andersen](https://github.com/rot256/swanky/tree/f4f9261a1f2ef7e338ab7a453fb450cc98801aac).[^1] The implementation of our VOLE protocol for $\mathbb{Z}_{2^k}$ is available in the subdirectory [`ocelot/src/ot/mozzarella`](ocelot/src/ot/mozzarella). The benchmarking code for the QuarkSilver zero-knowledge protocol is located in [`ocelot/src/quarksilver`](ocelot/src/quarksilver).

[^1]: The original README can be found [here](https://github.com/Pownieh/swanky/blob/0d66360cff270851ad9fecbeaeb8e06eee94d977/README.md).


## Compile

We have tested the code with Rust v1.58.1. It requires an x86 processor with AESNI and SSE2 instruction set extensions. To compile the benchmarks, run `cargo build --release`. Then the benchmark programs can be found under `target/release/mozzarella_bench` and `target/release/qs_mult_bench`.


## Running the Benchmarks

The benchmark binaries have a builtin `--help` which documents the available option.


### Example: VOLE Extension Benchmark

#### Sender / Prover Command
```sh
./target/release/mozzarella_bench \
--party prover \
--listen \
--host ::1 \
--threads=4 \
--repetitions=10 \
--json \
--ring=r144 \
--base-vole-size=553600 \
--num-noise-coordinates=2186 \
--extension-size=10557972
```

#### Receiver / Verifier Command
```sh
./target/release/mozzarella_bench \
--party verifier \
--host ::1 \
--threads=4 \
--repetitions=10 \
--json \
--ring=r144 \
--base-vole-size=553600 \
--num-noise-coordinates=2186 \
--extension-size=10557972
```
