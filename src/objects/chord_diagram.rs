use crate::*;
use crate::io::csv::*;

use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::str::FromStr;

use itertools::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChordDiagram {
	ends: Vec<u8>
}
impl Display for ChordDiagram {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "({})", self.ends.iter().map(|end| format!("{}", end)).join(", "))
	}
}
impl FromStr for ChordDiagram {
	type Err = ();
	fn from_str(mut s: &str) -> Result<Self, Self::Err> {
		// remove possible brackets
		if s.starts_with('(') && s.ends_with(')') {
			s = &s[1..s.len() - 1];
		}
		// the empty string
		if s.len() == 0 { return Ok(Self{ ends: vec![] }) }
		let ends = s.split(" ")
			.map(|a| a.parse::<u8>().map_err(|_| ()))
			.collect::<Result<Vec<_>, ()>>()?;
		Ok(Self { ends })
	}
}
impl CombCsv for ChordDiagram {
	type Err = ();
	const CSV_HEADER: &'static str = "chord diagram";
	fn to_csv_string(&self) -> String {
		format!("{}", self.ends.iter().map(|end| end.to_string()).join(", "))
	}
	fn from_csv_string<S: AsRef<str>>(s: S) -> Result<Self, Self::Err> {
		let s = s.as_ref();
		// the empty string
		if s.len() == 0 { return Ok(Self{ ends: vec![] }) }
		let ends = s.split(" ")
			.map(|a| a.parse::<u8>().map_err(|_| ()))
			.collect::<Result<Vec<_>, ()>>()?;
		Ok(Self { ends })
	}
}
impl CombGrad<usize> for ChordDiagram {
	fn degree(&self) -> usize { self.ends.len() / 2 }
}
impl CombEnum<usize> for ChordDiagram {
	type Iter = ChordDiagramIterator;
	fn iterate_deg(degree: usize) -> Self::Iter {
		ChordDiagramIterator::new(degree)
	}
}
impl CombCan for ChordDiagram {
	type Input = Vec<u8>;
	fn validate(ends: &Vec<u8>) -> bool { ends.len() % 2 == 0 }
	fn canonicalise(ends: &mut Vec<u8>) {
		let n = ends.len();
		*ends = (0..n)
			.map(|shift| ends[shift..].iter().chain(&ends[..shift]).cloned().collect::<Vec<u8>>())
			.map(|mut ends| {
				let mut mapping: HashMap<u8, u8> = HashMap::new();
				let mut k = 0;
				for i in 0..n {
					let x = ends[i];
					match mapping.get(&x) {
						Some(&y) => {
							ends[i] = y;
							mapping.remove(&x);
						}
						None => {
							ends[i] = k;
							mapping.insert(x, k);
							k += 1;
						}
					}
				}
				ends
			})
			.sorted()
			.next()
			// if `ends` is non-empty, we are guaranteed to get a value
			.unwrap_or(vec![]);
	}
	unsafe fn from_raw(ends: Vec<u8>) -> Self {
		Self { ends }
	}
}
impl ChordDiagram {
	pub fn ends(&self) -> &[u8] { &self.ends }
	pub fn apply<T, F: FnOnce(&mut Vec<u8>) -> T>(&mut self, f: F) -> T {
		let value = f(&mut self.ends);
		Self::canonicalise(&mut self.ends);
		value
	}
	pub fn neighbours(&self, chord: u8) -> Vec<u8> {
		let i_start = match self.ends.iter().position(|&a| a == chord) {
			Some(i_start) => { i_start }
			None => { return vec![]; }
		};
		let mut neighbours = HashSet::new();
		for &a in self.ends[i_start + 1..self.ends.len()].iter() {
			if a == chord { break; }
			if neighbours.contains(&a) {
				neighbours.remove(&a);
			} else {
				neighbours.insert(a);
			}
		}
		neighbours.into_iter().collect()
	}
	#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
	#[cfg(feature = "petgraph")]
	pub fn intersection_graph<Ix: petgraph::graph::IndexType>(&self) -> petgraph::graph::UnGraph<(), (), Ix> {
		let mut graph = petgraph::graph::UnGraph::default();
		let chords: Vec<u8> = self.ends.iter()
			.sorted()
			.dedup()
			.cloned()
			.collect();
		let nodes = chords.iter()
			.map(|&a| (a, graph.add_node(())))
			.collect::<HashMap<_, _>>();
		for &a in chords.iter() {
			for b in self.neighbours(a).into_iter().filter(|&b| a < b) {
				graph.add_edge(nodes[&a], nodes[&b], ());
			}
		}
		graph
	}
}

fn rooted_chord_diagrams(size: usize) -> Box<dyn Iterator<Item=Vec<u8>> + Sync + Send> {
	if size == 0 {
		return Box::new(std::iter::once(vec![]));
	} else {
		return Box::new(rooted_chord_diagrams(size - 1)
			.flat_map(move |mut diag| {
				diag = [0, 0].iter()
					.cloned()
					.chain(diag.into_iter().map(|x| x + 1))
					.collect();
				(1..2 * size)
					.map(move |i| {
						let mut diag = diag.clone();
						diag.swap(1, i);
						diag
					})
			})
		);
	}
}
fn binomial(n: u128, mut k: u128) -> u128 {
	if k > n { return 0; }
	k = u128::min(k, n - k);
	let mut c_nk = 1;
	for i in 1..=k {
		c_nk *= n + 1 - i;
		c_nk /= i;
	}
	c_nk
}
fn double_factorial(n: u128) -> u128 {
	// the case of overflow, this is actually -1
	if n == u128::MAX { return 1; }
	let k0 = if n % 2 == 0 { 2 } else { 1 };
	(k0..=n).step_by(2).product()
}
fn alpha(p: u128, q: u128) -> u128 {
	if q % 2 == 0 {
		(0..=p/2).map(|k| binomial(p, 2 * k) * q.pow(k as u32) * double_factorial((2 * k).overflowing_sub(1).0)).sum()
	} else {
		q.pow((p / 2) as u32) * double_factorial(p - 1)
	}
}
fn euler_phi(mut n: u128) -> u128 {
	let factorisation = primefactor::PrimeFactors::factorize(n as u128);
	let primes = factorisation.factors().iter().map(|factor| factor.integer);
	for p in primes {
		n -= n / p;
	}
	n
}
fn divisors(n: u128) -> Vec<u128> {
	use divisors::get_divisors;

	let mut divisors = vec![1];
	divisors.extend(get_divisors(n));
	if divisors.iter().last() != Some(&n) {
		divisors.push(n);
	}
	divisors
}
fn n_chord_diagrams(n: usize) -> usize {
	if n == 0 { return 1; }
	let count = divisors(2 * n as u128).into_iter()
		.map(|p| alpha(2 * n as u128 / p as u128, p as u128) * euler_phi(p as u128))
		.sum::<u128>() / (2 * n as u128);
	count as usize
}
pub struct ChordDiagramIterator {
	rooted: Box<dyn Iterator<Item=Vec<u8>> + Sync + Send>,
	cache: radix_trie::Trie<Vec<u8>, ()>,
	n_left: Option<usize>
}
impl ChordDiagramIterator {
	fn new(n: usize) -> Self {
		ChordDiagramIterator {
			rooted: rooted_chord_diagrams(n),
			cache: Default::default(),
			// The result cannot be stored in `u64` for `n >= 19`.
			// For `usize` < `u64`, the overflow happens even earlier.
			// We assume `usize` >= `u64`.
			n_left: if n <= 18 { Some(n_chord_diagrams(n)) } else { None }
		}
	}
}
impl Iterator for ChordDiagramIterator {
	type Item = ChordDiagram;
	fn next(&mut self) -> Option<Self::Item> {
		loop {
			if self.n_left == Some(0) { return None; }
			let mut ends = self.rooted.next()?;
			ChordDiagram::canonicalise(&mut ends);
			match self.cache.insert(ends.clone(), ()) {
				Some(()) => {}
				None => {
					self.n_left = self.n_left.map(|n_left| n_left - 1);
					return Some(ChordDiagram { ends });
				}
			}
		}
	}
	fn size_hint(&self) -> (usize, Option<usize>) {
		if let Some(n_left) = self.n_left {
			(n_left, Some(n_left))
		} else {
			(0, None)
		}
	}
}

#[cfg_attr(docsrs, doc(cfg(all(feature = "petgraph", feature = "rayon"))))]
#[cfg(all(feature = "petgraph", feature = "rayon"))]
pub fn intersection_graphs<Ix: petgraph::graph::IndexType + Send + Sync>(size: usize) -> impl Iterator<Item=(petgraph::graph::UnGraph<(), (), Ix>, ChordDiagram)> + Sync + Send {
	use crate::collections::*;
	ChordDiagram::iterate_deg(size)
		.map(|diag| (diag.intersection_graph(), diag))
		.filter({
			let mut graph_set = CombSet::<_>::new();
			move |(g, _)| {
				if graph_set.par_contains(g) {
					false
				} else {
					graph_set.insert_unchecked(g.clone());
					true
				}
			}
		})
}
#[cfg_attr(docsrs, doc(cfg(all(feature = "petgraph", not(feature = "rayon")))))]
#[cfg(all(feature = "petgraph", not(feature = "rayon")))]
pub fn intersection_graphs<Ix: petgraph::graph::IndexType>(size: usize) -> impl Iterator<Item=(petgraph::graph::UnGraph<(), (), Ix>, ChordDiagram)> {
	use crate::collections::*;
	ChordDiagram::iterate_deg(size)
		.map(|diag| (diag.intersection_graph(), diag))
		.filter({
			let mut graph_set = CombSet::<_>::new();
			move |(g, _)| {
				if graph_set.contains(g) {
					false
				} else {
					graph_set.insert_unchecked(g.clone());
					true
				}
			}
		})
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_chord_diagram_iterator() {
		for n in 0..=7 {
			assert_eq!(ChordDiagram::iterate_deg(n).count(), ChordDiagram::count_deg(n).unwrap());
		}
	}
	#[test]
	fn test_binomial() {
		let n = 10;
		let vals: Vec<u128> = (0..=n).map(|k| binomial(n, k)).collect();
		assert_eq!(vals, vec![1, 10, 45, 120, 210, 252, 210, 120, 45, 10, 1]);
	}
	#[test]
	fn test_double_factorial() {
		assert_eq!(double_factorial(0u128.overflowing_sub(1).0), 1);
		assert_eq!(double_factorial(0), 1);
		assert_eq!(double_factorial(1), 1);
		assert_eq!(double_factorial(16), 10321920);
		assert_eq!(double_factorial(17), 34459425);
	}
	#[test]
	fn test_euler_phi() {
		assert_eq!(euler_phi(1), 1);
		assert_eq!(euler_phi(2), 1);
		assert_eq!(euler_phi(12843), 8556);
		assert_eq!(euler_phi(1010102), 505050);
	}
	#[test]
	fn test_divisors() {
		assert_eq!(divisors(2u128), vec![1, 2]);
		assert_eq!(divisors(141u128), vec![1, 3, 47, 141]);
		assert_eq!(divisors(143u128), vec![1, 11, 13, 143]);
	}
	#[test]
	fn test_n_chord_diagrams() {
		// OEIS A007769
		let values = vec![
			1, 1, 2, 5, 18, 105, 902, 9749, 127072, 1915951,
			32743182, 624999093, 13176573910, 304072048265,
			7623505722158, 206342800616597, 5996837126024824,
			186254702826289089, 6156752656678674792
		];
		for n in 0..values.len() {
			assert_eq!(ChordDiagram::count_deg(n), Some(values[n]));
		}
	}
	#[cfg(feature = "petgraph")]
	#[test]
	fn test_intersection_graphs_iterator() {
		// OEIS A156809
		let values = vec![
			1, 1, 2, 4, 11, 34, 154, 978
		];
		for n in 0..values.len() {
			assert_eq!(intersection_graphs::<petgraph::graph::DefaultIx>(n).count(), values[n]);
		}
	}
}
