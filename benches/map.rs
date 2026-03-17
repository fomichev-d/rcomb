use rcomb::*;
use rcomb::collections::map::*;
#[cfg(feature = "rayon")]
use rcomb::rayon::iter::*;

use std::hint::black_box;
use std::time::*;

use criterion::{criterion_main, criterion_group, Criterion};


const N_ELEMENTS: usize = 500;
const SLEEP: Duration = Duration::from_micros(1);

fn busy_sleep(dur: Duration) -> Duration {
	let target = Instant::now() + dur;
	while Instant::now() < target {
		black_box(());
	}
	black_box(dur)
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct TestStruct(usize);
impl CombEq for TestStruct {
	fn hash(&self) -> Vec<usize> { vec![self.0 % 6] }
	fn is_isomorphic(&self, other: &Self) -> bool {
		black_box(busy_sleep(SLEEP));
		self == other
	}
}
impl CombGrad<usize> for TestStruct {
	fn degree(&self) -> usize { self.0 }
}

fn map_extend() -> CombMap<TestStruct, usize> {
	let mut map = CombMap::new();
	map.extend((0..N_ELEMENTS).map(|i| (TestStruct(i), i % 7)));
	map
}
#[cfg(feature = "rayon")]
fn map_par_extend() -> CombMap<TestStruct, usize> {
	let mut map = CombMap::new();
	map.par_extend((0..N_ELEMENTS).par_bridge().map(|i| (TestStruct(i), i % 7)));
	map
}
fn map_get(map: &mut CombMap<TestStruct, usize>) {
	for i in 0..N_ELEMENTS { black_box(map.get(&TestStruct(i))); }
}
#[cfg(feature = "rayon")]
fn map_par_get(map: &mut CombMap<TestStruct, usize>) {
	for i in 0..N_ELEMENTS { black_box(map.par_get(&TestStruct(i))); }
}

fn bench_map_extend(c: &mut Criterion) {
	let mut group = c.benchmark_group("map_extend");
	group.sample_size(1000);
	#[cfg(feature = "rayon")]
	group.bench_function("par", |b| b.iter(|| black_box(map_par_extend())));
	group.bench_function("seq", |b| b.iter(|| black_box(map_extend())));
}
fn bench_map_get(c: &mut Criterion) {
	let mut group = c.benchmark_group("map_get");
	group.sample_size(1000);
	let mut map = map_par_extend();
	#[cfg(feature = "rayon")]
	group.bench_function("par", |b| b.iter(|| black_box(map_par_get(&mut map))));
	group.bench_function("seq", |b| b.iter(|| black_box(map_get(&mut map))));
}

criterion_group! {
	name = benches;
	config = Criterion::default().significance_level(0.01).noise_threshold(0.02);
	targets = bench_map_extend, bench_map_get,
}
criterion_main!(benches);
