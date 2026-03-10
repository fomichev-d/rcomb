use rcomb::objects::*;
use rcomb::collections::*;

use std::hint::black_box;

use criterion::{criterion_main, criterion_group, Criterion};

#[derive(Clone, Copy, PartialEq, Eq)]
struct TestStruct(usize);
impl CombEq for TestStruct {
	fn hash(&self) -> Vec<usize> { vec![self.0 % 6] }
	fn is_isomorphic(&self, other: &Self) -> bool { self == other }
}
impl Grading<usize> for TestStruct {
	fn degree(&self) -> usize { self.0 }
}

fn map_insert_10000<M: CombMapBase<TestStruct, usize>>() {
	let mut map = M::new();
	map.extend((0..10000).map(|i| (TestStruct(i), i % 7)))
}

fn bench_map_insert_10000(c: &mut Criterion) {
	c.bench_function("bench_map_insert", |b| b.iter(|| black_box(map_insert_10000::<CombMap<_, _>>())));
}
#[cfg(feature = "rayon")]
fn bench_par_map_insert_10000(c: &mut Criterion) {
	c.bench_function("bench_par_map_insert", |b| b.iter(|| black_box(map_insert_10000::<CombParMap<_, _>>())));
}

#[cfg(not(feature = "rayon"))]
criterion_group!(
	benches,
	bench_map_insert_10000,
);
#[cfg(feature = "rayon")]
criterion_group!(
	benches,
	bench_map_insert_10000,
	bench_par_map_insert_10000,
);
criterion_main!(benches);
