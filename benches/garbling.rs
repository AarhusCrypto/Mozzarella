#[macro_use]
extern crate criterion;
extern crate fancy_garbling;

use criterion::Criterion;
use std::time::Duration;

use fancy_garbling::rand::Rng;
use fancy_garbling::garble::garble;
use fancy_garbling::circuit::{Builder, Circuit};
use fancy_garbling::util::IterToVec;

fn bench_garble<F:'static>(c: &mut Criterion, name: &str, make_circuit: F, q: u16)
    where F: Fn(u16) -> Circuit
{
    c.bench_function(&format!("garbling::{}{}_gb", name, q), move |bench| {
        let c = make_circuit(q);
        bench.iter(|| {
            let (gb, _ev) = garble(&c);
            criterion::black_box(gb);
        });
    });
}

fn bench_eval<F:'static>(c: &mut Criterion, name: &str, make_circuit: F, q: u16)
    where F: Fn(u16) -> Circuit
{
    c.bench_function(&format!("garbling::{}{}_ev", name, q), move |bench| {
        let ref mut rng = Rng::new();
        let c = make_circuit(q);
        let (gb, ev) = garble(&c);
        let inps = (0..c.ninputs()).map(|i| rng.gen_u16() % c.input_mod(i)).to_vec();
        let xs = gb.encode(&inps);
        bench.iter(|| {
            let ys = ev.eval(&c, &xs);
            criterion::black_box(ys);
        });
    });
}

fn proj(q: u16) -> Circuit {
    let mut tab = Vec::new();
    for i in 0..q {
        tab.push((i + 1) % q);
    }
    let mut b = Builder::new();
    let x = b.input(q);
    let z = b.proj(x, q, tab);
    b.output(z);
    b.finish()
}

fn half_gate(q: u16) -> Circuit {
    let mut b = Builder::new();
    let x = b.input(q);
    let y = b.input(q);
    let z = b.half_gate(x,y);
    b.output(z);
    b.finish()
}

fn proj17_gb(c: &mut Criterion) { bench_garble(c,"proj",proj,17) }
fn proj17_ev(c: &mut Criterion) { bench_eval(c,"proj",proj,17) }
fn mul_gb(c: &mut Criterion) { bench_garble(c,"mul",half_gate,17) }
fn mul_ev(c: &mut Criterion) { bench_eval(c,"mul",half_gate,17) }

criterion_group!{
    name = garbling;
    config = Criterion::default().warm_up_time(Duration::from_millis(100));
    targets = proj17_gb, proj17_ev, mul_gb, mul_ev
}

criterion_main!(garbling);
