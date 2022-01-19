use criterion::{criterion_group, criterion_main, Criterion};
use rand::{rngs::OsRng, Rng};
use scuttlebutt::ring::{z2r, NewRing, R64};
use std::time::Duration;

const SUM_SIZE: usize = 100_000;

fn bench_r64_sum_iter(c: &mut Criterion) {
    c.bench_function("R64: sum with iter", |b| {
        let values: Vec<R64> = (0..SUM_SIZE).into_iter().map(|_| OsRng.gen()).collect();
        b.iter(|| {
            let sum: R64 = values.iter().copied().sum();
            criterion::black_box(sum)
        });
    });
}

fn bench_r64_sum_slice(c: &mut Criterion) {
    c.bench_function("R64: sum with slice", |b| {
        let values: Vec<R64> = (0..SUM_SIZE).into_iter().map(|_| OsRng.gen()).collect();
        b.iter(|| {
            let mut s = 0u64;
            for x in values.as_slice() {
                s += x.0;
            }
            let sum = R64(s);
            criterion::black_box(sum)
        });
    });
}

fn bench_z2r_128_sum_iter(c: &mut Criterion) {
    c.bench_function("Z2r<104>: sum with iter", |b| {
        let values: Vec<z2r::R104> = (0..SUM_SIZE).into_iter().map(|_| OsRng.gen()).collect();
        b.iter(|| {
            let sum: z2r::R104 = values.iter().copied().sum();
            criterion::black_box(sum)
        });
    });
}

fn bench_z2r_128_sum_slice(c: &mut Criterion) {
    c.bench_function("Z2r<104>: sum with slice", |b| {
        let values: Vec<z2r::R104> = (0..SUM_SIZE).into_iter().map(|_| OsRng.gen()).collect();
        b.iter(|| {
            let sum = z2r::R104::sum(values.as_slice());
            criterion::black_box(sum)
        });
    });
}

fn bench_z2r_192_sum_iter(c: &mut Criterion) {
    c.bench_function("Z2rU192<144>: sum with iter", |b| {
        let values: Vec<z2r::Z2rU192<144>> =
            (0..SUM_SIZE).into_iter().map(|_| OsRng.gen()).collect();
        b.iter(|| {
            let sum: z2r::Z2rU192<144> = values.iter().copied().sum();
            criterion::black_box(sum)
        });
    });
}

fn bench_z2r_192_sum_slice(c: &mut Criterion) {
    c.bench_function("Z2rU192<144>: sum with slice", |b| {
        let values: Vec<z2r::Z2rU192<144>> =
            (0..SUM_SIZE).into_iter().map(|_| OsRng.gen()).collect();
        b.iter(|| {
            let sum = z2r::Z2rU192::<144>::sum(values.as_slice());
            criterion::black_box(sum)
        });
    });
}

fn bench_z2r_256_sum_iter(c: &mut Criterion) {
    c.bench_function("Z2rU256<144>: sum with iter", |b| {
        let values: Vec<z2r::Z2rU256<144>> =
            (0..SUM_SIZE).into_iter().map(|_| OsRng.gen()).collect();
        b.iter(|| {
            let sum: z2r::Z2rU256<144> = values.iter().copied().sum();
            criterion::black_box(sum)
        });
    });
}

fn bench_z2r_256_sum_slice(c: &mut Criterion) {
    c.bench_function("Z2rU256<144>: sum with slice", |b| {
        let values: Vec<z2r::Z2rU256<144>> =
            (0..SUM_SIZE).into_iter().map(|_| OsRng.gen()).collect();
        b.iter(|| {
            let sum = z2r::Z2rU256::<144>::sum(values.as_slice());
            criterion::black_box(sum)
        });
    });
}

criterion_group! {
    name = z2r;
    config = Criterion::default().warm_up_time(Duration::from_millis(100));
    targets = bench_r64_sum_iter, bench_r64_sum_slice, bench_z2r_128_sum_iter, bench_z2r_128_sum_slice, bench_z2r_192_sum_iter, bench_z2r_192_sum_slice, bench_z2r_256_sum_iter, bench_z2r_256_sum_slice
}
criterion_main!(z2r);
