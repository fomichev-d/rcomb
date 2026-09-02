use crate::collections::set::CombSet;
use crate::objects::multigraph::MGraph;
use crate::*;
use crate::objects::graph::*;
use crate::objects::cyclic_order::*;

use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::ops::Range;

use fixedbitset::FixedBitSet;
use itertools::Itertools;
use petgraph::visit::EdgeRef;
use petgraph::visit::{Data, GetAdjacencyMatrix, GraphBase, IntoNeighbors, IntoNodeIdentifiers, NodeIndexable};


pub struct EmbGraph<V=(), HE=(), E=()> {
	/// (CyclicOrder(half_edge_id), vertex_data)
	pub vertices: Vec<(CyclicOrder<usize>, V)>,
	/// (vertex_id, edge_id, half_edge_data)
	pub half_edges: Vec<(usize, usize, HE)>,
	/// (source_half_edge_id, target_half_edge_id, edge_data)
	pub edges: Vec<(usize, usize, E)>
}
impl<V: Clone, HE: Clone, E: Clone> Clone for EmbGraph<V, HE, E> {
	fn clone(&self) -> Self {
		Self { vertices: self.vertices.clone(), half_edges: self.half_edges.clone(), edges: self.edges.clone() }
	}
}
impl<V: Debug, HE: Debug, E: Debug> Debug for EmbGraph<V, HE, E> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "EmbGraph {{ vertices: {:?}, half_edges: {:?}, edges: {:?} }}", self.vertices, self.half_edges, self.edges)
	}
}
impl<V: Display, HE: Display, E: Display> Display for EmbGraph<V, HE, E> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f,
			"{{ vertices: [{}], half_edges: [{}], edges: [{}] }}",
			self.vertices.iter().enumerate().map(|(i, v)| format!("v{}({}, {})", i, v.0, v.1)).join(", "),
			self.half_edges.iter().enumerate().map(|(i, he)| format!("he{}(v{}, e{}, {})", i, he.0, he.1, he.2)).join(", "),
			self.edges.iter().enumerate().map(|(i, e)| format!("e{}(he{}-he{}, {})", i, e.0, e.1, e.2)).join(", ")
		)
	}
}
impl<V: PartialEq, HE: PartialEq, E: PartialEq> PartialEq for EmbGraph<V, HE, E> {
	fn eq(&self, other: &Self) -> bool {
		self.vertices == other.vertices && self.half_edges == other.half_edges && self.edges == other.edges
	}
}
impl<V: Eq, HE: Eq, E: Eq> Eq for EmbGraph<V, HE, E> {}
impl<V, HE, E> Default for EmbGraph<V, HE, E> {
	fn default() -> Self {
		Self { vertices: vec![], half_edges: vec![], edges: vec![] }
	}
}
impl EmbGraph {
	pub fn build(adj: Vec<Vec<usize>>) -> Self {
		let n = adj.len();
		let e = adj.iter()
			.flat_map(|order| order)
			.max()
			.map(|&e| e + 1)
			.unwrap_or(0);
		let edges = (0..e)
			.map(|e| (2 * e, 2 * e + 1, ()))
			.collect();
		let mut half_edges: Vec<(usize, usize, ())> = (0..2 * e)
			.map(|he| (0, he / 2, ()))
			.collect();
		let mut vertices: Vec<(CyclicOrder<usize>, ())> = (0..n)
			.map(|_| (CyclicOrder::default(), ()))
			.collect();
		let mut he_shift = vec![0; e];
		for v in 0..n {
			let mut data = vec![];
			for &e in &adj[v] {
				data.push(2 * e + he_shift[e]);
				half_edges[2 * e + he_shift[e]].0 = v;
				he_shift[e] = (he_shift[e] + 1) % 2;
			}
			let order = CyclicOrder { data };
			vertices[v].0 = order;
		}
		Self { vertices, half_edges, edges }
	}

	pub fn from_multi(graph: MGraph) -> Self {
		let mut g = Self::new(graph.num_verts(), |_| ());
		for e in graph.graph.0.edge_references() {
			for _ in 0..*e.weight() {
				g.add_edge(
					e.source().index(), None, (),
					e.target().index(), None, (),
					()
				);
			}
		}
		g
	}
}
impl<V, HE, E> EmbGraph<V, HE, E> {
	pub fn new<F: Fn(usize) -> V>(n: usize, v_data: F) -> Self {
		Self { vertices: (0..n).map(|k| (CyclicOrder::default(), v_data(k))).collect(), half_edges: vec![], edges: vec![] }
	}

	pub fn validate(&self) {
		for (e, &(source, target, _)) in self.edges.iter().enumerate() {
			assert!(source < self.half_edges.len(), "e {}: bad he {}", e, source);
			assert!(target < self.half_edges.len(), "e {}: bad he {}", e, target);
			assert_eq!(self.half_edges[source].1, e, "e/he correspondence corrupted at {}/{}", e, source);
			assert_eq!(self.half_edges[target].1, e, "e/he correspondence corrupted at {}/{}", e, target);
		}
		for (he, &(v, _, _)) in self.half_edges.iter().enumerate() {
			assert!(v < self.num_verts(), "he {}: bad v {}", he, v);
			assert!(self.vertices[v].0.data.contains(&he), "he/v correspondence corrupted at {}/{}", he, v);
		}
		for he in 0..self.half_edges.len() {
			let k: usize = self.vertices.iter()
				.map(|(order, _)| order.data.iter().filter(|&&x| x == he).count())
				.sum();
			assert_eq!(k, 1, "unexpected # of he {}: got {} != 1", he, k);
		}
	}

	/// returns old value
	pub fn update_vertex(&mut self, v: usize, v_data: V) -> V {
		std::mem::replace(&mut self.vertices[v].1, v_data)
	}
	/// returns old value
	pub fn update_half_edge(&mut self, he: usize, he_data: HE) -> HE {
		std::mem::replace(&mut self.half_edges[he].2, he_data)
	}
	/// returns old value
	pub fn update_edge(&mut self, e: usize, e_data: E) -> E {
		std::mem::replace(&mut self.edges[e].2, e_data)
	}

	pub fn v(&self, v: usize) -> &V { &self.vertices[v].1 }
	pub fn he(&self, he: usize) -> &HE { &self.half_edges[he].2 }
	pub fn e(&self, e: usize) -> &E { &self.edges[e].2 }
	pub fn he_vertex(&self, he: usize) -> usize { self.half_edges[he].0 }
	pub fn he_edge(&self, he: usize) -> usize { self.half_edges[he].1 }
	pub fn he_other(&self, he: usize) -> usize {
		let e = self.he_edge(he);
		let (u_he, v_he) = (self.edges[e].0, self.edges[e].1);
		return he ^ u_he ^ v_he;
	}
	pub fn he_next(&self, he: usize) -> usize {
		let he_other = self.he_other(he);
		let v_other = self.he_vertex(he_other);
		self.vertices[v_other].0.next(he_other)
	}

	pub fn num_verts(&self) -> usize { self.vertices.len() }
	pub fn num_edges(&self) -> usize { self.edges.len() }

	pub fn neighbours(&self, v: usize) -> Vec<usize> {
		self.vertices[v].0.data.iter()
			.map(|&he| (he, self.half_edges[he].1))
			.map(|(he, e)| (he, self.edges[e].0, self.edges[e].1))
			.map(|(he, he1, he2)| he ^ he1 ^ he2)
			.map(|he| self.half_edges[he].0)
			.collect()
	}

	pub fn add_vertex(&mut self, v_data: V) -> usize {
		let v = self.vertices.len();
		self.vertices.push((CyclicOrder::default(), v_data));
		return v;
	}
	pub fn add_edge(
		&mut self,
		u: usize, after_u_he: Option<usize>, u_he_data: HE,
		v: usize, after_v_he: Option<usize>, v_he_data: HE,
		e_data: E
	) {
		let u_he = self.half_edges.len();
		let v_he = u_he + 1;
		let e = self.edges.len();
		self.vertices[u].0.insert(u_he, after_u_he);
		self.vertices[v].0.insert(v_he, after_v_he);
		self.half_edges.push((u, e, u_he_data));
		self.half_edges.push((v, e, v_he_data));
		self.edges.push((u_he, v_he, e_data));
	}
	pub fn remove_vertex(&mut self, v: usize) -> V {
		for e in self.vertices[v].0.data.iter().map(|&he| self.he_edge(he)).collect::<Vec<_>>() {
			self.remove_edge(e);
		}
		let v_last = self.vertices.len() - 1;
		
		let (_, v_data) = self.vertices.swap_remove(v);
		if v != v_last {
			for (u, _, _) in self.half_edges.iter_mut() {
				if *u == v_last { *u = v; }
			}
		}
		v_data
	}
	fn remove_half_edge(&mut self, he: usize) -> HE {
		let he_last = self.half_edges.len() - 1;

		let (v, _, he_data) = self.half_edges.swap_remove(he);
		self.vertices[v].0.remove(he);
		if he != he_last {
			let v_last = self.he_vertex(he);
			let e_last = self.he_edge(he);
			self.vertices[v_last].0.replace(he_last, he);

			if self.edges[e_last].0 == he_last {
				self.edges[e_last].0 = he;
			} else if self.edges[e_last].1 == he_last {
				self.edges[e_last].1 = he;
			} else {
				panic!("he{} not found in e{}!", he_last, e_last);
			}
		}
		he_data
	}
	pub fn remove_edge(&mut self, e: usize) -> E {
		let e_last = self.edges.len() - 1;

		let (u_he, v_he, e_data) = self.edges.swap_remove(e);
		self.remove_half_edge(u_he);
		self.remove_half_edge(v_he);
		if e != e_last {
			self.half_edges[self.edges[e].0].1 = e;
			self.half_edges[self.edges[e].1].1 = e;
		}
		e_data
	}
	pub fn vertex_degree(&self, v: usize) -> usize {
		self.vertices[v].0.len()
	}
	pub fn map<
		V2, HE2, E2,
		F: FnMut(usize, &V) -> V2,
		G: FnMut(usize, &HE) -> HE2,
		H: FnMut(usize, &E) -> E2
	>(&self, mut v_map: F, mut he_map: G, mut e_map: H) -> EmbGraph<V2, HE2, E2> {
		let vertices = self.vertices.iter()
			.enumerate()
			.map(|(k, (order, val))| (order.clone(), v_map(k, val)))
			.collect();
		let half_edges = self.half_edges.iter()
			.enumerate()
			.map(|(k, (v, e, val))| (*v, *e, he_map(k, val)))
			.collect();
		let edges = self.edges.iter()
			.enumerate()
			.map(|(k, (source, target, val))| (*source, *target, e_map(k, val)))
			.collect();

		EmbGraph { vertices, half_edges, edges }
	}
	pub fn filter_map<
		V2, HE2, E2,
		F: FnMut(usize, &V) -> Option<V2>,
		G: FnMut(usize, &HE) -> HE2,
		H: FnMut(usize, &E) -> Option<E2>
	>(&self, mut v_map: F, mut he_map: G, mut e_map: H) -> EmbGraph<V2, HE2, E2> {
		// (CyclicOrder(half_edge_id), vertex_data)
		let mut vertices = vec![];
		// (vertex_id, edge_id, half_edge_data)
		let mut half_edges = vec![];
		// (source_half_edge_id, target_half_edge_id, edge_data)
		let mut edges = vec![];

		let mut vmap = HashMap::new();
		for v in 0..self.num_verts() {
			if let Some(val) = v_map(v, &self.vertices[v].1) {
				let k = vmap.len();
				vmap.insert(v, k);
				vertices.push((v, val));
			}
		}
		let mut emap = HashMap::new();
		for e in 0..self.num_edges() {
			let (source, target, _) = self.edges[e];
			let (v, u) = (self.he_vertex(source), self.he_vertex(target));
			if !vmap.contains_key(&v) || !vmap.contains_key(&u) { continue; }
			if let Some(val) = e_map(e, &self.edges[e].2) {
				let k = emap.len();
				emap.insert(e, k);
				edges.push((e, val));
			}
		}
		let mut hemap = HashMap::new();
		for (he, (v, e, data)) in self.half_edges.iter().enumerate() {
			if let Some(&v) = vmap.get(v) && let Some(&e) = emap.get(e) {
				let val = he_map(he, data);
				let k = half_edges.len();
				hemap.insert(he, k);
				half_edges.push((v, e, val));
			}
		}

		let vertices = vertices.into_iter()
			.map(|(v, val)| (
				self.vertices[v].0.filter_map(|l| hemap.get(l).copied()),
				val
			))
			.collect();
		let edges = edges.into_iter()
			.map(|(e, val)| (hemap[&self.edges[e].0], hemap[&self.edges[e].1], val))
			.collect();

		EmbGraph { vertices, half_edges, edges }
	}
	pub fn vertex_subgraph<F: FnMut(usize, &V) -> bool>(&self, mut f: F) -> Self where V: Clone, HE: Clone, E: Clone {
		self.filter_map(
			|k, x| {
				if f(k, x) {
					Some(x.clone())
				} else {
					None
				}
			},
			|_, val| { val.clone() },
			|_, val| { Some(val.clone()) }
		)
	}
	pub fn edge_subgraph<F: FnMut(usize, &E) -> bool>(&self, mut f: F) -> Self where V: Clone, HE: Clone, E: Clone {
		self.filter_map(
			|_, val| { Some(val.clone()) },
			|_, val| { val.clone() },
			|k, x| {
				if f(k, x) {
					Some(x.clone())
				} else {
					None
				}
			}
		)
	}
	pub fn connected_components_subgraphs(&self) -> Vec<Self> where V: Clone, HE: Clone, E: Clone {
		let mut comps = vec![];
		let mut scc = petgraph::algo::TarjanScc::new();
		scc.run(&self, |comp: &[usize]| {
			let h = self.vertex_subgraph(|v, _| { comp.contains(&v) });
			comps.push(h);
		});
		comps
	}
	pub fn connected_component_number(&self) -> usize {
		let mut n = 0;
		let mut scc = petgraph::algo::TarjanScc::new();
		scc.run(&self, |_| { n += 1; });
		n
	}
}

impl<V, HE, E> GraphHash for EmbGraph<V, HE, E> {
	// TODO: a better invariant, account for cycle lengths
	fn graph_hash(&self) -> Vec<usize> {
		let n = self.num_verts();
		let mut data = vec![0; n + (n + 1) * 1];
		// first n entries: #verts with i neighbours
		let adj = self.adjacency_matrix();
		for v in 0..n {
			let deg = (0..n)
				.filter(|&u| self.is_adjacent(&adj, v, u))
				.count();
			data[deg] += 1;
		}
		// second n+1 entries: #verts with i 2-neighbours
		let adj_ref = &adj;
		for v in 0..n {
			let deg2 = (0..n)
				.filter(|&u| self.is_adjacent(&adj, v, u))
				.flat_map(|u| (0..n).filter(move |&w| self.is_adjacent(adj_ref, w, u)))
				.sorted()
				.dedup()
				.count();
			data[n + deg2] += 1;
		}
		data
	}
}
// assumption: g1, g2 are connected, not empty
fn emb_graph_match(g1: &EmbGraph, g2: &EmbGraph, mut hemap: HashMap<usize, usize>, mut hemap_unchecked: HashMap<usize, usize>) -> bool {
	if hemap.len() == g1.half_edges.len() {
		return true;
	}
	if hemap_unchecked.len() == 0 {
		// find first match
		let v = (0..g1.num_verts())
			.min_by_key(|&v| g1.vertex_degree(v))
			.unwrap();
		let order = &g1.vertices[v].0;
		let deg = g1.vertex_degree(v);
		for u in 0..g2.num_verts() {
			if g2.vertex_degree(u) != deg { continue; }
			if deg == 0 { return true; }
			let u_order = &g2.vertices[u].0;
			for offset in 0..deg {
				let mut hemap_unchecked = HashMap::new();
				for i in 0..deg {
					hemap_unchecked.insert(order.data[i], u_order.data[(i + offset) % deg]);
				}
				if emb_graph_match(g1, g2, hemap.clone(), hemap_unchecked) {
					return true;
				}
			}
		}
		return false;
	}
	// match a cycle
	let he0 = hemap_unchecked.keys()
		.next()
		.cloned()
		.and_then(|k| hemap_unchecked.remove_entry(&k))
		.unwrap();
	hemap.insert(he0.0, he0.1);
	let mut he = (g1.he_next(he0.0), g2.he_next(he0.1));
	while he.0 != he0.0 {
		// the cycle in g2 should not be shorter
		// without it g1=6-cycle and g2=3-cycle will be determined isomorphic
		if he.1 == he0.1 { return false; }
		if let Some(dst) = hemap_unchecked.remove(&he.0) {
			if he.1 != dst { return false; }
		} else {
			let v = g1.he_vertex(he.0);
			let v_order = &g1.vertices[v].0;
			let u = g2.he_vertex(he.1);
			let u_order = &g2.vertices[u].0;
			if v_order.len() != u_order.len() { return false; }
			for he1 in v_order.iter_from(he.0).zip(u_order.iter_from(he.1)).skip(1) {
				hemap_unchecked.insert(he1.0, he1.1);
			}
		}
		hemap.insert(he.0, he.1);
		he = (g1.he_next(he.0), g2.he_next(he.1));
	}
	if he.1 != he0.1 { return false; }
	emb_graph_match(g1, g2, hemap, hemap_unchecked)
}
impl CombEq for EmbGraph {
	fn hash(&self) -> Vec<usize> {
		self.graph_hash()
	}
	fn is_isomorphic(&self, other: &Self) -> bool {
		let mut comps = self.connected_components_subgraphs();
		let mut other_comps = other.connected_components_subgraphs();
		if comps.len() != other_comps.len() { return false; }
		while let Some(comp) = comps.pop() {
			let hash = comp.hash();
			let mut found: Option<usize> = None;
			for (i, other_comp) in other_comps.iter().enumerate() {
				if hash != other_comp.hash() { continue; }
				if emb_graph_match(&comp, other_comp, HashMap::default(), HashMap::default()) {
					found = Some(i);
					break;
				}
			}
			if let Some(i) = found {
				other_comps.remove(i);
			} else {
				return false;
			}
		}
		true
	}
}
impl<V, HE, E> CombGrad for EmbGraph<V, HE, E> {
	fn degree(&self) -> usize {
		self.num_verts()
	}
}

impl<V, HE, E> CombGrad<JacobiDeg> for EmbGraph<V, HE, E> {
	fn degree(&self) -> JacobiDeg {
		debug_assert_eq!(self.num_verts() % 2, 0);
		JacobiDeg(self.num_verts() / 2)
	}
}
impl CombEnum<JacobiDeg> for EmbGraph {
	type Iter = Box<dyn Iterator<Item=Self> + Sync + Send>;
	fn iterate_deg_inner(degree: JacobiDeg) -> Self::Iter {
		Box::new(
			MGraph::iterate_deg_inner(degree)
				.map(|graph| Self::from_multi(graph))
				// permutations of half-edge orders
				.flat_map(|base| {
					let graphs: CombSet<_> = base.vertices.iter()
						.enumerate()
						.filter(|(_, (order, ()))| order.len() > 1)
						.map(|(v, (order, ()))| {
							order.permutations().map(move |order| (v, order))
						})
						.multi_cartesian_product()
						.map(|changes| {
							let mut g = base.clone();
							for (v, order) in changes {
								g.vertices[v].0 = order;
							}
							g
						})
						.collect();
					graphs
				})
		)
	}
	fn count_deg(_degree: JacobiDeg) -> Option<usize> {
		// TODO
		None
	}
}

impl<V, HE, E> GraphBase for EmbGraph<V, HE, E> {
	type NodeId = usize;
	type EdgeId = usize;
}
impl<V, HE, E> Data for EmbGraph<V, HE, E> {
	type NodeWeight = V;
	type EdgeWeight = E;
}
impl<V, HE, E> NodeIndexable for EmbGraph<V, HE, E> {
	fn node_bound(&self) -> usize { self.num_verts() }
	fn to_index(&self, a: Self::NodeId) -> usize { a }
	fn from_index(&self, i: usize) -> Self::NodeId { i }
}
impl<V, HE, E> IntoNodeIdentifiers for &EmbGraph<V, HE, E> {
	type NodeIdentifiers = Range<usize>;
	fn node_identifiers(self) -> Self::NodeIdentifiers {
		0..self.num_verts()
	}
}
impl<V, HE, E> IntoNeighbors for &EmbGraph<V, HE, E> {
	type Neighbors = std::vec::IntoIter<usize>;
	fn neighbors(self, a: Self::NodeId) -> Self::Neighbors {
		self.neighbours(a).into_iter()
	}
}
impl<V, HE, E> GetAdjacencyMatrix for EmbGraph<V, HE, E> {
	type AdjMatrix = FixedBitSet;
	fn adjacency_matrix(&self) -> Self::AdjMatrix {
		let n = self.num_verts();
		let mut matrix = FixedBitSet::with_capacity(n * n);

		for e in 0..self.num_edges() {
			let (e_i, e_j, _) = self.edges[e];
			let i = self.half_edges[e_i].0;
			let j = self.half_edges[e_j].0;
			matrix.insert(i * n + j);
			matrix.insert(j * n + i);
		}
		matrix
	}
	fn is_adjacent(&self, matrix: &Self::AdjMatrix, a: Self::NodeId, b: Self::NodeId) -> bool {
		let n = self.num_verts();
		matrix.contains(a * n + b)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_count_jacobi() {
		let values = vec![
			1,
			1, 15, 227, 4311, 96559, 2632903, // 85532247,
		];
		for n in 0..values.len() {
			let degree = JacobiDeg(n);
			assert_eq!(EmbGraph::iterate_deg(degree).count(), values[n], "at {}", n);
		}
	}

	#[test]
	fn connected_components() {
		let g = EmbGraph::build(vec![
			vec![0],
			vec![0],
			vec![1],
			vec![1]
		]);
		g.validate();
		eprintln!("{:?}", g);
		g.connected_components_subgraphs();
	}

	#[test]
	fn deg2_noniso() {
		let g1 = EmbGraph::build(vec![
			vec![0],
			vec![0, 2, 1],
			vec![3, 1, 2],
			vec![3]
		]);
		g1.validate();
		eprintln!("{:?}", g1);

		let g2 = EmbGraph::build(vec![
			vec![0],
			vec![0, 2, 1],
			vec![3, 2, 1],
			vec![3]
		]);
		g2.validate();
		eprintln!("{:?}", g2);

		assert!(!g1.is_isomorphic(&g2));
		assert!(!g2.is_isomorphic(&g1));
	}
}
