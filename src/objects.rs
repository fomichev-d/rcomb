#[cfg(feature = "petgraph")]
use petgraph::graph::{UnGraph, IndexType};
#[cfg(feature = "petgraph")]
use petgraph::visit::GetAdjacencyMatrix;
#[cfg(feature = "petgraph")]
use itertools::*;

pub trait CombEq {
	fn hash(&self) -> Vec<usize>;
	fn is_isomorphic(&self, other: &Self) -> bool;
}

pub trait Grading<T: Copy + Eq + Ord + Send + Sync = usize> {
	fn degree(&self) -> T;
}

pub trait CombEnum<T: Copy + Eq + Ord + Send + Sync>: Grading<T> {
	type Iter: Iterator<Item=Self>;
	fn iterate_deg(degree: T) -> Self::Iter;
	fn count_deg(_degree: T) -> Option<usize> { None }
}

// petgraph integration

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
#[cfg(feature = "petgraph")]
impl<N, E, Ix: IndexType> Grading<usize> for UnGraph<N, E, Ix> {
	fn degree(&self) -> usize { self.node_count() }
}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
#[cfg(feature = "petgraph")]
pub trait GraphHash {
	fn graph_hash(&self) -> Vec<usize>;
}
#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
#[cfg(feature = "petgraph")]
impl<N, E, Ix: IndexType> GraphHash for UnGraph<N, E, Ix> {
	fn graph_hash(&self) -> Vec<usize> {
		let n = self.node_count();
		let mut data = vec![0; n + (n + 1) * 1];
		// first n entries: #verts with i neighbours
		let adj = self.adjacency_matrix();
		for v in self.node_indices() {
			let deg = self.node_indices()
				.filter(|&u| self.is_adjacent(&adj, v, u))
				.count();
			data[deg] += 1;
		}
		// second n+1 entries: #verts with i 2-neighbours
		let adj_ref = &adj;
		for v in self.node_indices() {
			let deg2 = self.node_indices()
				.filter(|&u| self.is_adjacent(&adj, v, u))
				.flat_map(|u| self.node_indices().filter(move |&w| self.is_adjacent(adj_ref, w, u)))
				.sorted()
				.dedup()
				.count();
			data[n + deg2] += 1;
		}
		// performance costs are significant here
		/*
		// third n+1 entries: #verts with i 3-neighbours
		let adj_ref = &adj;
		for v in self.node_indices() {
			let deg3 = self.node_indices()
				.filter(|&u| self.is_adjacent(&adj, v, u))
				.flat_map(|u| self.node_indices().filter(move |&w| self.is_adjacent(adj_ref, w, u)))
				.flat_map(|u| self.node_indices().filter(move |&w| self.is_adjacent(adj_ref, w, u)))
				.sorted()
				.dedup()
				.count();
			data[2 * n + 1 + deg3] += 1;
		}
		*/
		data
	}
}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
#[cfg(feature = "petgraph")]
pub trait NodeMatch: Eq {}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
#[cfg(feature = "petgraph")]
pub trait EdgeMatch: Eq {}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
#[cfg(feature = "petgraph")]
impl<Ix: IndexType> CombEq for UnGraph<(), (), Ix> {
	fn hash(&self) -> Vec<usize> {
		self.graph_hash()
	}
	/// Neither graph must contains multiple edges.
	fn is_isomorphic(&self, other: &Self) -> bool {
		petgraph::algo::is_isomorphic(self, other)
	}
}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
#[cfg(feature = "petgraph")]
impl<N: NodeMatch, Ix: IndexType> CombEq for UnGraph<N, (), Ix> {
	fn hash(&self) -> Vec<usize> {
		self.graph_hash()
	}
	/// Neither graph must contains multiple edges.
	fn is_isomorphic(&self, other: &Self) -> bool {
		petgraph::algo::is_isomorphic_matching(self, other, |v1, v2| { v1 == v2 }, |_, _| { true })
	}
}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
#[cfg(feature = "petgraph")]
impl<E: EdgeMatch, Ix: IndexType> CombEq for UnGraph<(), E, Ix> {
	fn hash(&self) -> Vec<usize> {
		self.graph_hash()
	}
	/// Neither graph must contains multiple edges.
	fn is_isomorphic(&self, other: &Self) -> bool {
		petgraph::algo::is_isomorphic_matching(self, other, |_, _| { true }, |e1, e2| { e1 == e2 })
	}
}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
#[cfg(feature = "petgraph")]
impl<N: NodeMatch, E: EdgeMatch, Ix: IndexType> CombEq for UnGraph<N, E, Ix> {
	fn hash(&self) -> Vec<usize> {
		self.graph_hash()
	}
	/// Neither graph must contains multiple edges.
	fn is_isomorphic(&self, other: &Self) -> bool {
		petgraph::algo::is_isomorphic_matching(self, other, |v1, v2| { v1 == v2 }, |e1, e2| { e1 == e2 })
	}
}
