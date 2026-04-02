use rcomb::objects::graph::Graph;
use rcomb::*;

use std::hint::black_box;

use criterion::{criterion_main, criterion_group, Criterion};

const N: usize = 8;

fn iter_geng(n: usize) -> usize {
	Graph::iterate_deg(n).count()
}

fn bench_geng(c: &mut Criterion) {
	let mut group = c.benchmark_group("graph_iter");
	group.sample_size(1000);
	group.bench_function("geng", |b| b.iter(|| black_box(iter_geng(N))));
}
criterion_group! {
	name = benches;
	config = Criterion::default().significance_level(0.01).noise_threshold(0.02);
	targets = bench_geng
}
criterion_main!(benches);
