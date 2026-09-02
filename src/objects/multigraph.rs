use crate::io::CombCsv;
use crate::*;
use crate::objects::graph::*;

#[cfg(feature = "geng")]
use std::io::{BufReader, BufRead};
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::num::ParseIntError;
use std::ops::*;

use petgraph::graph::{EdgeIndex, EdgeReference, EdgeReferences, Neighbors, NodeIndex, NodeReferences, UnGraph};
use petgraph::graph6::FromGraph6;
use petgraph::stable_graph::StableUnGraph;
use petgraph::visit::EdgeRef;
use itertools::*;

#[derive(Clone, Debug)]
pub struct MGraph<V = ()> {
	/// The underlying graph.
	/// It has multiple edges, but is only used as a skeleton.
	pub graph: Graph<V, usize>
}
impl<V> Default for MGraph<V> {
	fn default() -> Self {
		Self { graph: Graph::default() }
	}
}

impl<V> From<UnGraph<V, ()>> for MGraph<V> {
	fn from(value: UnGraph<V, ()>) -> Self {
		let graph = Graph::from(value.map_owned(|_, v| { v }, |_, _| { 1 }));
		Self { graph }
	}
}
impl<V> From<StableUnGraph<V, ()>> for MGraph<V> {
	fn from(value: StableUnGraph<V, ()>) -> Self {
		let graph = Graph::from(value.map_owned(|_, v| { v }, |_, _| { 1 }));
		Self { graph }
	}
}

impl FromGraph6 for MGraph {
	#[inline]
	fn from_graph6_string(graph6_string: String) -> Self {
		let graph = Graph::from(
			UnGraph::from_graph6_string(graph6_string)
				.map_owned(|_, v| { v }, |_, _| { 1 })
		);
		Self { graph }
	}
}

impl<V> Index<NodeIndex> for MGraph<V> {
	type Output = V;
	fn index(&self, index: NodeIndex) -> &Self::Output {
		&self.graph[index]
	}
}
impl<V> IndexMut<NodeIndex> for MGraph<V> {
	fn index_mut(&mut self, index: NodeIndex) -> &mut Self::Output {
		&mut self.graph[index]
	}
}

impl<V> MGraph<V> {
	#[inline]
	pub fn num_verts(&self) -> usize { self.graph.num_verts() }
	#[inline]
	pub fn num_edges(&self) -> usize {
		self.graph.0.edge_references()
			.map(|e| e.weight())
			.sum()
	}
	#[inline]
	pub fn vertices(&self) -> NodeReferences<'_, V> {
		self.graph.vertices()
	}
	#[inline]
	pub fn neighbours(&self, v: NodeIndex) -> Neighbors<'_, usize> {
		self.graph.neighbours(v)
	}
	#[inline]
	pub fn edges(&self) -> std::iter::Map<EdgeReferences<'_, usize>, fn(EdgeReference<usize>) -> (NodeIndex, NodeIndex)> {
		self.graph.edges()
	}
	#[inline]
	pub fn add_vertex_with(&mut self, weight: V) -> NodeIndex {
		self.graph.add_vertex_with(weight)
	}
	#[inline]
	pub fn delete_vertex(&mut self, u: NodeIndex) -> Option<V> {
		self.graph.delete_vertex(u)
	}
	#[inline]
	pub fn vertex_degree(&self, v: NodeIndex) -> usize {
		self.graph.0.edges(v).map(|e| *e.weight()).sum()
	}
	#[inline]
	pub fn add_edge(&mut self, u: NodeIndex, v: NodeIndex) {
		if let Some(e) = self.graph.0.find_edge(u, v) {
			self.graph.0[e] += 1;
		} else {
			self.graph.0.add_edge(u, v, 1);
		}
	}
	#[inline]
	pub fn add_multiedge(&mut self, u: NodeIndex, v: NodeIndex, n: usize) {
		if let Some(e) = self.graph.0.find_edge(u, v) {
			self.graph.0[e] += n;
		} else {
			self.graph.0.add_edge(u, v, n);
		}
	}
	#[inline]
	pub fn delete_edge(&mut self, u: NodeIndex, v: NodeIndex) -> Option<()> {
		let e = self.graph.0.find_edge(u, v)?;
		self.graph.0[e] -= 1;
		if self.graph.0[e] == 0 {
			self.graph.0.remove_edge(e);
		}
		Some(())
	}
	#[inline]
	pub fn has_edge(&self, u: NodeIndex, v: NodeIndex) -> bool {
		self.graph.has_edge(u, v)
	}
	#[inline]
	pub fn map<
		V2,
		F: FnMut(NodeIndex, &V) -> V2,
	>(&self, vertex_map: F) -> MGraph<V2> {
		let graph = self.graph.map(vertex_map, |_, &n| { n });
		MGraph { graph }
	}
	#[inline]
	pub fn filter_map<
		V2,
		F: FnMut(NodeIndex, &V) -> Option<V2>,
		G: FnMut(EdgeIndex, usize) -> usize
	>(&self, vertex_map: F, mut edge_map: G) -> MGraph<V2> {
		let graph = self.graph.filter_map(
			vertex_map,
			move |e, &n| {
				let n = edge_map(e, n);
				if n > 0 { Some(n) } else { None }
			}
		);
		MGraph { graph }
	}
	#[inline]
	pub fn vertex_subgraph<F: FnMut(NodeIndex, &V) -> bool>(&self, mut f: F) -> Self where V: Clone {
		self.filter_map(
			|v, v_type| {
				if f(v, v_type) {
					Some(v_type.clone())
				} else {
					None
				}
			},
			|_, e_type| { e_type }
		)
	}
	#[inline]
	pub fn edge_subgraph<F: FnMut(EdgeIndex, usize) -> usize>(&self, f: F) -> Self where V: Clone {
		self.filter_map(
			|_, v_type| { Some(v_type.clone()) },
			f
		)
	}
	pub fn connected_components_subgraphs(&self) -> Vec<Self> where V: Clone {
		self.graph.connected_components_subgraphs()
			.into_iter()
			.map(|graph| Self { graph })
			.collect()
	}
	pub fn connected_component_number(&self) -> usize {
		self.graph.connected_component_number()
	}
	// TODO: make it a concrete type with automatic Send+Sync?
	pub fn edge_subgraphs(&self) -> impl Iterator<Item=Self> + Send + Sync where V: Clone + Send + Sync {
		self.graph.0.edge_indices()
			.flat_map(|e| (0..=self.graph.0[e]).map(move |n| (e, n)))
			.powerset()
			.map(|edges| edges.into_iter().collect::<HashMap<_, _>>())
			.map(|edges| self.edge_subgraph(|e, _| edges[&e]))
	}

}
impl MGraph {
	#[inline]
	pub fn add_vertex(&mut self) -> NodeIndex {
		self.graph.add_vertex()
	}
}
impl Display for MGraph {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{{{} [", self.num_verts())?;
		for (i, e) in self.graph.0.edge_references().enumerate() {
			if i > 0 {
				write!(f, ", ")?;
			}
			write!(f, "({}, {}, {})", e.source().index(), e.target().index(), e.weight())?;
		}
		write!(f, "]}}")
	}
}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
impl<V> CombGrad<usize> for MGraph<V> {
	fn degree(&self) -> usize { self.graph.degree() }
}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
impl<V> CombGrad<JacobiDeg> for MGraph<V> {
	fn degree(&self) -> JacobiDeg {
		JacobiDeg(self.graph.num_verts() / 2)
	}
}

#[cfg_attr(docsrs, doc(cfg(all(feature = "petgraph", feature = "geng"))))]
#[cfg(feature = "geng")]
impl CombEnum<JacobiDeg> for MGraph {
	type Iter = Box<dyn Iterator<Item=Self> + Sync + Send>;
	fn iterate_deg_inner(degree: JacobiDeg) -> Self::Iter {
		let JacobiDeg(degree) = degree;
		let n = degree * 2;
		if n == 0 {
			return Box::new(std::iter::once(MGraph::default()));
		}
		let mut geng = std::process::Command::new("geng");
		let geng_stdout = geng
			.arg("-q")
			.arg(n.to_string())
			.arg("-d1")
			.arg("-D3")
			.stdout(std::process::Stdio::piped())
			.stderr(std::process::Stdio::null())
			.spawn()
			.expect("geng failed")
			.stdout
			.expect("geng failed");
		let mut multig = std::process::Command::new("multig");
		let multig_stdout = multig
			.arg("-q")
			.arg("-T")
			.arg("-D3")
			.stdin(geng_stdout)
			.stdout(std::process::Stdio::piped())
			.spawn()
			.expect("multig failed")
			.stdout
			.expect("multig failed");
		Box::new(
			BufReader::new(multig_stdout)
				.lines()
				.filter_map(|l| l.ok())
				.filter(|l| l.len() > 0)
				.map(|l| l.split_whitespace()
					.flat_map(|x| x.parse::<usize>())
					.collect::<Vec<_>>()
				)
				.map(|data| (
					data[0], data[1],
					data[2..].into_iter()
						.copied()
						.tuples::<(_, _, _)>()
						.collect::<Vec<_>>()
				))
				.map(|(n, _, data)| {
					let mut g = MGraph::default();
					let verts = (0..n)
						.map(|_| g.add_vertex())
						.collect::<Vec<_>>();
					for (v1, v2, mult) in data {
						g.add_multiedge(verts[v1], verts[v2], mult);
					}
					g
				})
				// filter out graphs with vertex degree 2
				.filter(move |graph| {
					graph.vertices()
						.map(|(v, _)| graph.vertex_degree(v))
						.all(|deg| deg == 1 || deg == 3)
				})
				// each connected component must have a degree 1 vertex
				.filter(|graph| {
					let mut scc = petgraph::algo::TarjanScc::new();
					let mut good = true;
					scc.run(&graph.graph.0, |comp: &[NodeIndex]| {
						good &= comp.iter().any(|&v| {
							graph.vertex_degree(v) == 1
						});
					});
					good
				})
		)
	}
	// TODO: implement it properly
	fn count_deg(degree: JacobiDeg) -> Option<usize> {
		Some(Self::iterate_deg_inner(degree).count())
	}
}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
impl CombCsv for MGraph {
	type Err = ParseIntError;
	const CSV_HEADER: &'static str = "multig-text";
	fn to_csv_string(&self) -> String {
		let n = self.num_verts();
		// does not count multiedges as several by design
		let e = self.graph.num_edges();
		[n, e].into_iter()
			.chain(self.graph.0.edge_references()
				.flat_map(|e| {
					[e.source().index(), e.target().index(), *e.weight()]
				})
			)
			.map(|k| k.to_string())
			.join(" ")
	}
	fn from_csv_string<S: AsRef<str>>(s: S) -> Result<Self, Self::Err> {
		let data = s.as_ref().split_whitespace()
			.map(|x| x.parse::<usize>())
			.collect::<Result<Vec<_>, _>>()?;
		let n = data[0];
		let data: Vec<_> = data.into_iter()
			.skip(2)
			.tuples::<(_, _, _)>()
			.collect();
		let mut g = MGraph::default();
		let verts = (0..n)
			.map(|_| g.add_vertex())
			.collect::<Vec<_>>();
		for (v1, v2, mult) in data {
			g.add_multiedge(verts[v1], verts[v2], mult);
		}
		Ok(g)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	#[test]
	fn jacobi_count_deg() {
		let values = vec![
			1,
			1, 4, 15, 72, 402, 2714, 21720, // 205863,
		];
		for n in 0..values.len() {
			let degree = JacobiDeg(n);
			assert_eq!(MGraph::count_deg(degree), Some(values[n]), "at {}", n);
		}
	}
}
