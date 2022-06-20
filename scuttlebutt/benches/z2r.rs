use criterion::{criterion_group, criterion_main, Criterion};
use rand::{rngs::OsRng, Rng};
use scuttlebutt::ring::{z2r, Ring, R64};
use std::time::Duration;

const SUM_SIZE: usize = 100_000;

fn u192_add(c: &mut Criterion) {
    c.bench_function("u192_add", |b| {
        let x: z2r::Z2rU192::<144> = rand::thread_rng().gen();
        let y: z2r::Z2rU192::<144> = rand::thread_rng().gen();
        b.iter(|| criterion::black_box(criterion::black_box(x) + criterion::black_box(y)));
    });
}

fn u192_mul(c: &mut Criterion) {
    c.bench_function("u192_mul", |b| {
        let x: z2r::Z2rU192::<144> = rand::thread_rng().gen();
        let y: z2r::Z2rU192::<144> = rand::thread_rng().gen();
        b.iter(|| criterion::black_box(criterion::black_box(x) * criterion::black_box(y)));
    });
}

fn u192_sum(c: &mut Criterion) {
    c.bench_function("u192_sum10", |b| {
        let x: Vec<z2r::Z2rU192::<144>> = (0..10)
            .map(|_| rand::thread_rng().gen())
            .collect();
        b.iter(|| criterion::black_box(criterion::black_box(x.iter().copied()).sum::<z2r::Z2rU192::<144>>()));
    });
    c.bench_function("u192_sum100", |b| {
        let x: Vec<z2r::Z2rU192::<144>> = (0..100)
            .map(|_| rand::thread_rng().gen())
            .collect();
        b.iter(|| criterion::black_box(criterion::black_box(x.iter().copied()).sum::<z2r::Z2rU192::<144>>()));
    });
    c.bench_function("u192_sum1000", |b| {
        let x: Vec<z2r::Z2rU192::<144>> = (0..1000)
            .map(|_| rand::thread_rng().gen())
            .collect();
        b.iter(|| criterion::black_box(criterion::black_box(x.iter().copied()).sum::<z2r::Z2rU192::<144>>()));
    });
    c.bench_function("u192_sum1M", |b| {
        let x: Vec<z2r::Z2rU192::<144>> = (0..1000000)
            .map(|_| rand::thread_rng().gen())
            .collect();
        b.iter(|| criterion::black_box(criterion::black_box(x.iter().copied()).sum::<z2r::Z2rU192::<144>>()));
    });
}

fn u192_mulvec(c: &mut Criterion) {
    c.bench_function("u192_mulvec1M", |b| {
        let x: Vec<z2r::Z2rU192<144>> = (0..1000000).map(|_| rand::thread_rng().gen()).collect();
        let y: Vec<z2r::Z2rU192<144>> = (0..1000000).map(|_| rand::thread_rng().gen()).collect();
        // let mut z = vec![z2r::Z2rU192<144>::default(); 1000000];
        b.iter(|| {
            criterion::black_box(
                criterion::black_box(x.iter().copied())
                    .zip(criterion::black_box(y.iter().copied()))
                    .map(|(xi, yi)| xi * yi)
                    .collect::<Vec<z2r::Z2rU192<144>>>(),
            )
        });
    })
    ;
}

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
    config = Criterion::default().warm_up_time(Duration::from_millis(100)).sample_size(10).without_plots();
    targets = bench_r64_sum_iter, bench_r64_sum_slice, bench_z2r_128_sum_iter, bench_z2r_128_sum_slice, bench_z2r_192_sum_iter, bench_z2r_192_sum_slice, bench_z2r_256_sum_iter, bench_z2r_256_sum_slice, u192_add, u192_mul, u192_sum, u192_mulvec
}
criterion_main!(z2r);
