use crate::*;
use crate::io::*;

use std::fmt::Display;
#[cfg(feature = "geng")]
use std::io::{BufReader, BufRead};
use std::ops::{Index, IndexMut};

use petgraph::graph::{EdgeIndex, EdgeReference, EdgeReferences, Neighbors, NodeIndex, NodeReferences, UnGraph, IndexType};
use petgraph::graph6::*;
use petgraph::prelude::StableUnGraph;
use petgraph::visit::{EdgeRef, GetAdjacencyMatrix, IntoNodeReferences};
use itertools::*;

// petgraph integration

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
#[derive(Clone, Debug)]
pub struct Graph<V = (), E = ()>(pub(crate) UnGraph<V, E>);
impl<V, E> Default for Graph<V, E> {
	fn default() -> Self { Self(UnGraph::default()) }
}

impl<V, E> From<UnGraph<V, E>> for Graph<V, E> {
	fn from(value: UnGraph<V, E>) -> Self { Self(value) }
}
impl<V, E> From<StableUnGraph<V, E>> for Graph<V, E> {
	fn from(value: StableUnGraph<V, E>) -> Self { Self(value.into()) }
}
impl FromGraph6 for Graph {
	#[inline]
	fn from_graph6_string(graph6_string: String) -> Self {
		Graph(UnGraph::from_graph6_string(graph6_string))
	}
}
impl ToGraph6 for Graph {
	#[inline]
	fn graph6_string(&self) -> String {
		self.0.graph6_string()
	}
}
impl<V, E> From<Graph<V, E>> for UnGraph<V, E> {
	fn from(value: Graph<V, E>) -> Self {
		value.0
	}
}
impl<V, E> From<Graph<V, E>> for StableUnGraph<V, E> {
	fn from(value: Graph<V, E>) -> Self {
		Self::from(value.0)
	}
}

impl<V, E> Index<NodeIndex> for Graph<V, E> {
	type Output = V;
	fn index(&self, index: NodeIndex) -> &Self::Output {
		self.0.node_weight(index).expect("vertex not found")
	}
}
impl<V, E> IndexMut<NodeIndex> for Graph<V, E> {
	fn index_mut(&mut self, index: NodeIndex) -> &mut Self::Output {
		self.0.node_weight_mut(index).expect("vertex not found")
	}
}
impl<V, E> Index<(NodeIndex, NodeIndex)> for Graph<V, E> {
	type Output = E;
	fn index(&self, index: (NodeIndex, NodeIndex)) -> &Self::Output {
		let idx = self.0.find_edge(index.0, index.1);
		idx.map(|e| &self.0[e]).expect("edge not found")
	}
}
impl<V, E> IndexMut<(NodeIndex, NodeIndex)> for Graph<V, E> {
	fn index_mut(&mut self, index: (NodeIndex, NodeIndex)) -> &mut Self::Output {
		let idx = self.0.find_edge(index.0, index.1);
		idx.map(|e| &mut self.0[e]).expect("edge not found")
	}
}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
impl<V, E> Graph<V, E> {
	#[inline]
	pub fn num_verts(&self) -> usize { self.0.node_count() }
	#[inline]
	pub fn num_edges(&self) -> usize { self.0.edge_count() }
	#[inline]
	pub fn vertices(&self) -> NodeReferences<'_, V> {
		self.0.node_references()
	}
	#[inline]
	pub fn neighbours(&self, v: NodeIndex) -> Neighbors<'_, E> {
		self.0.neighbors(v)
	}
	#[inline]
	pub fn edges(&self) -> std::iter::Map<EdgeReferences<'_, E>, fn(EdgeReference<E>) -> (NodeIndex, NodeIndex)> {
		self.0.edge_references()
			.map(|e| (e.source(), e.target()))
	}
	#[inline]
	pub fn add_vertex_with(&mut self, weight: V) -> NodeIndex {
		self.0.add_node(weight)
	}
	#[inline]
	pub fn delete_vertex(&mut self, u: NodeIndex) -> Option<V> {
		self.0.remove_node(u)
	}
	#[inline]
	pub fn vertex_degree(&self, v: NodeIndex) -> usize {
		self.0.neighbors(v).count()
	}
	#[inline]
	pub fn add_edge_with(&mut self, u: NodeIndex, v: NodeIndex, weight: E) {
		self.0.add_edge(u, v, weight);
	}
	#[inline]
	pub fn delete_edge(&mut self, u: NodeIndex, v: NodeIndex) -> Option<E> {
		let idx = self.0.find_edge(u, v);
		idx.map(|e| self.0.remove_edge(e)).flatten()
	}
	#[inline]
	pub fn has_edge(&self, u: NodeIndex, v: NodeIndex) -> bool {
		self.0.find_edge(u, v).is_some()
	}
	#[inline]
	pub fn map<
		V2,
		E2,
		F: FnMut(NodeIndex, &V) -> V2,
		G: FnMut(EdgeIndex, &E) -> E2
	>(&self, vertex_map: F, edge_map: G) -> Graph<V2, E2> {
		Graph(self.0.map(vertex_map, edge_map))
	}
	#[inline]
	pub fn filter_map<
		V2,
		E2,
		F: FnMut(NodeIndex, &V) -> Option<V2>,
		G: FnMut(EdgeIndex, &E) -> Option<E2>
	>(&self, vertex_map: F, edge_map: G) -> Graph<V2, E2> {
		Graph(self.0.filter_map(vertex_map, edge_map))
	}
	#[inline]
	pub fn vertex_subgraph<F: FnMut(NodeIndex, &V) -> bool>(&self, mut f: F) -> Self where V: Clone, E: Clone {
		self.filter_map(
			|v, v_type| {
				if f(v, v_type) {
					Some(v_type.clone())
				} else {
					None
				}
			},
			|_, e_type| { Some(e_type.clone()) }
		)
	}
	#[inline]
	pub fn edge_subgraph<F: FnMut(EdgeIndex, &E) -> bool>(&self, mut f: F) -> Self where V: Clone, E: Clone {
		self.filter_map(
			|_, v_type| { Some(v_type.clone()) },
			|e, e_type| { 
					if f(e, e_type) {
					Some(e_type.clone())
				} else {
					None
				}
			}
		)
	}
	pub fn connected_components_subgraphs(&self) -> Vec<Self> where V: Clone, E: Clone {
		let mut comps = vec![];
		let mut scc = petgraph::algo::TarjanScc::new();
		scc.run(&self.0, |comp: &[NodeIndex]| {
			let h = self.vertex_subgraph(|v, _| { comp.contains(&v) });
			comps.push(h);
		});
		comps
	}
	pub fn connected_component_number(&self) -> usize {
		petgraph::algo::connected_components(&self.0)
	}
	// TODO: make it a concrete type with automatic Send+Sync?
	pub fn edge_subgraphs(&self) -> impl Iterator<Item=Self> + Send + Sync where V: Clone + Send + Sync, E: Clone + Send + Sync {
		self.0.edge_indices()
			.powerset()
			.map(|edges| self.edge_subgraph(|e, _| edges.contains(&e)))
	}

}
impl<V> Graph<V, ()> {
	#[inline]
	pub fn add_edge(&mut self, u: NodeIndex, v: NodeIndex) {
		self.0.add_edge(u, v, ());
	}
	#[inline]
	pub fn switch_edge(&mut self, u: NodeIndex, v: NodeIndex) {
		if let Some(e) = self.0.find_edge(u, v) {
			self.0.remove_edge(e);
		} else {
			self.0.add_edge(u, v, ());
		}
	}
}
impl<E> Graph<(), E> {
	#[inline]
	pub fn add_vertex(&mut self) -> NodeIndex {
		self.0.add_node(())
	}
}
impl Display for Graph {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{{{} [", self.num_verts())?;
		for (i, e) in self.edges().enumerate() {
			if i > 0 {
				write!(f, ", ")?;
			}
			write!(f, "({}, {})", e.0.index(), e.1.index())?;
		}
		write!(f, "]}}")
	}
}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
impl<V, E> CombGrad<usize> for Graph<V, E> {
	fn degree(&self) -> usize { self.0.node_count() }
}

#[cfg_attr(docsrs, doc(cfg(all(feature = "petgraph", feature = "geng"))))]
#[cfg(feature = "geng")]
impl CombEnum<usize> for Graph {
	type Iter = Box<dyn Iterator<Item=Graph> + Sync + Send>;
	// TODO: a proper implementation
	fn iterate_deg_inner(degree: usize) -> Self::Iter {
		if degree == 0 { return Box::new(std::iter::once(Graph::default())); }
		let stdout = std::process::Command::new("geng")
			.arg("-q")
			.arg(format!("{}", degree))
			.stdout(std::process::Stdio::piped())
			.spawn()
			.expect("geng failed")
			.stdout
			.expect("geng failed");
		Box::new(
			BufReader::new(stdout)
				.lines()
				.filter_map(|l| l.ok())
				.filter(|l| l.len() > 0)
				.map(|graph6| Graph(UnGraph::from_graph6_string(graph6)))
		)
	}
	// TODO: implement it properly
	fn count_deg(degree: usize) -> Option<usize> {
		Some(Self::iterate_deg_inner(degree).count())
	}
}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
pub trait GraphHash {
	fn graph_hash(&self) -> Vec<usize>;
}
#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
impl<V, E> GraphHash for Graph<V, E> {
	fn graph_hash(&self) -> Vec<usize> {
		self.0.graph_hash()
	}
}
impl<V, E, Ix: IndexType> GraphHash for UnGraph<V, E, Ix> {
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
		data
	}
}
impl<V, E, Ix: IndexType> GraphHash for StableUnGraph<V, E, Ix> {
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
		data
	}
}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
pub trait NodeMatch: Eq {}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
pub trait EdgeMatch: Eq {}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
impl CombEq for Graph {
	fn hash(&self) -> Vec<usize> {
		self.graph_hash()
	}
	fn is_isomorphic(&self, other: &Self) -> bool {
		petgraph::algo::is_isomorphic(&self.0, &other.0)
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
impl<Ix: IndexType> CombEq<Graph> for UnGraph<(), (), Ix> {
	fn hash(&self) -> Vec<usize> {
	    self.graph_hash()
	}
	fn is_isomorphic(&self, other: &Graph) -> bool {
		petgraph::algo::is_isomorphic(&self, &other.0)
	}
}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
impl<N: NodeMatch> CombEq for Graph<N, ()> {
	fn hash(&self) -> Vec<usize> {
		self.graph_hash()
	}
	fn is_isomorphic(&self, other: &Self) -> bool {
		petgraph::algo::is_isomorphic_matching(&self.0, &other.0, |v1, v2| { v1 == v2 }, |_, _| { true })
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
impl<N: NodeMatch, Ix: IndexType> CombEq<Graph<N, ()>> for UnGraph<N, (), Ix> {
	fn hash(&self) -> Vec<usize> {
	    self.graph_hash()
	}
	fn is_isomorphic(&self, other: &Graph<N, ()>) -> bool {
		petgraph::algo::is_isomorphic_matching(&self, &other.0, |v1, v2| { v1 == v2 }, |_, _| { true })
	}
}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
impl<E: EdgeMatch> CombEq for Graph<(), E> {
	fn hash(&self) -> Vec<usize> {
		self.graph_hash()
	}
	fn is_isomorphic(&self, other: &Self) -> bool {
		petgraph::algo::is_isomorphic_matching(&self.0, &other.0, |_, _| { true }, |e1, e2| { e1 == e2 })
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
impl<E: EdgeMatch, Ix: IndexType> CombEq<Graph<(), E>> for UnGraph<(), E, Ix> {
	fn hash(&self) -> Vec<usize> {
	    self.graph_hash()
	}
	fn is_isomorphic(&self, other: &Graph<(), E>) -> bool {
		petgraph::algo::is_isomorphic_matching(&self, &other.0, |_, _| { true }, |e1, e2| { e1 == e2 })
	}
}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
impl<N: NodeMatch, E: EdgeMatch> CombEq for Graph<N, E> {
	fn hash(&self) -> Vec<usize> {
		self.graph_hash()
	}
	fn is_isomorphic(&self, other: &Self) -> bool {
		petgraph::algo::is_isomorphic_matching(&self.0, &other.0, |v1, v2| { v1 == v2 }, |e1, e2| { e1 == e2 })
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
impl<N: NodeMatch, E: EdgeMatch, Ix: IndexType> CombEq<Graph<N, E>> for UnGraph<N, E, Ix> {
	fn hash(&self) -> Vec<usize> {
	    self.graph_hash()
	}
	fn is_isomorphic(&self, other: &Graph<N, E>) -> bool {
		petgraph::algo::is_isomorphic_matching(&self, &other.0, |v1, v2| { v1 == v2 }, |e1, e2| { e1 == e2 })
	}
}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
impl CombCsv for Graph {
	type Err = ();
	const CSV_HEADER: &'static str = "graph6";
	fn to_csv_string(&self) -> String {
		self.graph6_string()
	}
	fn from_csv_string<S: AsRef<str>>(s: S) -> Result<Self, Self::Err> {
		Ok(Self::from_graph6_string(s.as_ref().into()))
	}
}
