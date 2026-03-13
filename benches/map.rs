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
#[cfg(all(feature = "petgraph", feature = "rayon"))]
fn load_graphs() -> Vec<petgraph::graph::UnGraph<(), ()>> {
	use rcomb::rayon::iter::{ParallelBridge, ParallelIterator};
	use std::io::BufRead;
	use petgraph::graph6::FromGraph6;
	let file = std::fs::File::open("benches/graphs9.txt").unwrap();
	let reader = std::io::BufReader::new(file);
	reader.lines().map(|l| l.unwrap())
		.par_bridge()
		.map(|line| petgraph::graph::UnGraph::from_graph6_string(String::from(line)))
		.collect()
}
#[cfg(all(feature = "petgraph", not(feature = "rayon")))]
fn load_graphs() -> Vec<petgraph::graph::UnGraph<(), ()>> {
	use std::io::BufRead;
	use petgraph::graph6::FromGraph6;
	let file = std::fs::File::open("benches/graphs9.txt").unwrap();
	let reader = std::io::BufReader::new(file);
	reader.lines().map(|l| l.unwrap())
		.map(|line| petgraph::graph::UnGraph::from_graph6_string(String::from(line)))
		.filter(|g| g.degree() == 9)
		.collect()
}

fn map_insert_many<M: CombMapBase<TestStruct, usize>>() -> M {
	let mut map = M::new();
	map.extend((0..5000).map(|i| (TestStruct(i), i % 7)));
	map
}
fn map_get_many<M: CombMapBase<TestStruct, usize>>(map: &mut M) {
	for i in 0..5000 { black_box(map.get(&TestStruct(i))); }
}
#[cfg(feature = "petgraph")]
fn map_insert_graphs<M: CombMapBase<petgraph::graph::UnGraph<(), ()>, usize>>(graphs: &[petgraph::graph::UnGraph<(), ()>]) -> M {
	let mut map = M::new();
	map.extend(graphs.iter().map(|g| (g.clone(), g.edge_count())));
	map
}
#[cfg(feature = "petgraph")]
fn map_get_graphs<M: CombMapBase<petgraph::graph::UnGraph<(), ()>, usize>>(map: &mut M) {
	for g in map.keys().filter(|g| g.degree() == 9) { black_box(map.get(g)); }
}

fn bench_map_insert_many(c: &mut Criterion) {
	let mut group = c.benchmark_group("map_insert_many");
	#[cfg(feature = "rayon")]
	group.bench_function("par", |b| b.iter(|| black_box(map_insert_many::<CombParMap<_, _>>())));
	group.bench_function("seq", |b| b.iter(|| black_box(map_insert_many::<CombMap<_, _>>())));
}
fn bench_map_get_many(c: &mut Criterion) {
	let mut group = c.benchmark_group("map_get_many");
	#[cfg(feature = "rayon")]
	let mut map_par = map_insert_many::<CombParMap<_, _>>();
	let mut map_seq = map_insert_many::<CombMap<_, _>>();
	#[cfg(feature = "rayon")]
	group.bench_function("par", |b| b.iter(|| black_box(map_get_many(&mut map_par))));
	group.bench_function("seq", |b| b.iter(|| black_box(map_get_many(&mut map_seq))));
}

#[cfg(feature = "petgraph")]
fn bench_map_insert_graphs(c: &mut Criterion) {
	let mut group = c.benchmark_group("map_insert_graphs");
	group.sample_size(20);
	let graphs = load_graphs();
	#[cfg(feature = "rayon")]
	group.bench_function("par", |b| b.iter(|| black_box(map_insert_graphs::<CombParMap<_, _>>(&graphs))));
	group.bench_function("seq", |b| b.iter(|| black_box(map_insert_graphs::<CombMap<_, _>>(&graphs))));
}
#[cfg(feature = "petgraph")]
fn bench_map_get_graphs(c: &mut Criterion) {
	let mut group = c.benchmark_group("map_get_graphs");
	let graphs = load_graphs();
	#[cfg(feature = "rayon")]
	let mut map_par = map_insert_graphs::<CombParMap<_, _>>(&graphs);
	let mut map_seq = map_insert_graphs::<CombMap<_, _>>(&graphs);
	#[cfg(feature = "rayon")]
	group.bench_function("par", |b| b.iter(|| black_box(map_get_graphs(&mut map_par))));
	group.bench_function("seq", |b| b.iter(|| black_box(map_get_graphs(&mut map_seq))));
}

#[cfg(not(feature = "petgraph"))]
criterion_group! {
	name = benches;
	config = Criterion::default().significance_level(0.01).noise_threshold(0.02);
	targets = bench_map_insert_many, bench_map_get_many,
}
#[cfg(feature = "petgraph")]
criterion_group! {
	name = benches;
	config = Criterion::default().significance_level(0.01).noise_threshold(0.02);
	targets = bench_map_insert_many, bench_map_get_many, bench_map_insert_graphs, bench_map_get_graphs,
}
criterion_main!(benches);
