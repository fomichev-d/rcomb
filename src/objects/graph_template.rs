use crate::*;
use crate::objects::graph::*;
use crate::collections::set::*;

use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::str::FromStr;

use petgraph::{graph::NodeIndex, prelude::StableUnGraph};
use itertools::Itertools;
#[cfg(feature = "rayon")]
use rayon::iter::{ParallelBridge, ParallelIterator};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum GraphTemplateVertex {
	Free,
	Group(u8)
}
impl NodeMatch for GraphTemplateVertex {}

pub type GraphTemplate = Graph<GraphTemplateVertex>;

impl Display for GraphTemplate {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		fn format_vertex(v: NodeIndex, v_type: GraphTemplateVertex, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			match v_type {
				GraphTemplateVertex::Free => {
					write!(f, "{}", v.index())
				}
				GraphTemplateVertex::Group(g_id) => {
					write!(f, "g{g_id}")
				}
			}
		}
		fn format_edge(g: &GraphTemplate, e: (NodeIndex, NodeIndex), f: &mut std::fmt::Formatter) -> std::fmt::Result {
			let (u, v) = (e.0, e.1);
			let (u_type, v_type) = (g[u], g[v]);
			write!(f, "(")?;
			format_vertex(u, u_type, f)?;
			write!(f, ", ")?;
			format_vertex(v, v_type, f)?;
			write!(f, ")")
		}
		write!(f, "{{ [")?;
		for (i, (v, &v_type)) in self.vertices().enumerate() {
			if i > 0 {
				write!(f, ", ")?;
			}
			format_vertex(v, v_type, f)?;
		}
		write!(f, "] [")?;
		for (i, e) in self.edges().enumerate() {
			if i > 0 {
				write!(f, ", ")?;
			}
			format_edge(self, e, f)?;
		}
		write!(f, "]}}")
	}
}
impl FromStr for GraphTemplate {
	type Err = std::fmt::Error;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		// TODO: error handling?
		let n: usize = s[s.find('{').unwrap() + 1..s.find(' ').unwrap()]
			.parse()
			.unwrap();
		let mut free: HashSet<usize> = HashSet::new();
		let mut groups: HashSet<String> = HashSet::new();
		let edge_data = &s[s.find('[').unwrap() + 1..s.find(']').unwrap()];
		let mut edges: Vec<(&str, &str)> = vec![];
		if edge_data.len() > 0 {
			let mut register_vertex = |a: &str| {
				if a.starts_with('g') {
					groups.insert(a.into());
				} else {
					free.insert(a.parse().unwrap());
				}
			};
			for edge_str in edge_data[edge_data.find('(').unwrap() + 1..edge_data.rfind(')').unwrap()].split("), (") {
				let (a, b) = edge_str.split(", ").collect_tuple().unwrap();
				register_vertex(a);
				register_vertex(b);
				edges.push((a, b));
			}
		}
		while free.len() + groups.len() < n {
			let i = (0..n).filter(|i| !free.contains(i))
				.next().unwrap();
			free.insert(i);
		}
		let mut vertices: HashMap<String, (usize, GraphTemplateVertex)> = HashMap::new();
		for i in free { vertices.insert(i.to_string(), (i, GraphTemplateVertex::Free)); }
		for i in groups {
			let j = (0..n).filter(|j| vertices.values().all(|(k, _)| k != j))
				.next().unwrap();
			vertices.insert(i.to_string(), (j, GraphTemplateVertex::Group(i[1..].parse().unwrap())));
		}
		let edges: Vec<(usize, usize)> = edges.into_iter()
			.map(|(a, b)| { (vertices[a].0, vertices[b].0) })
			.collect();
		let vertices: HashMap<usize, GraphTemplateVertex> = vertices.into_values().collect();
		let mut graph: GraphTemplate = Graph::default();
		let mut node_map: Vec<NodeIndex> = vec![];
		for i in 0..n {
			node_map.push(graph.add_vertex_with(vertices[&i]));
		}
		for (a, b) in edges {
			graph.add_edge(node_map[a], node_map[b]);
		}
		Ok(graph)
	}
}

impl GraphTemplate {
	pub fn new(g: Graph, mapping: &[GraphTemplateVertex]) -> Self {
		g.map(
			|v, _| { mapping[v.index()] },
			|_, _| {}
		)
	}

	pub fn free_verts(&self) -> impl Iterator<Item=NodeIndex> + Send + Sync {
		self.vertices()
			.filter(|&(_, &v_type)| v_type == GraphTemplateVertex::Free)
			.map(|(v, _)| v)
	}

	pub fn groups(&self) -> Vec<GraphTemplateVertex> {
		self.vertices()
			.map(|(_, &v_type)| v_type)
			.filter(|&v_type| v_type != GraphTemplateVertex::Free)
			.collect()
	}

	pub fn can_merge(&self, g1: GraphTemplateVertex, g2: GraphTemplateVertex) -> bool {
		if g1 == GraphTemplateVertex::Free || g2 == GraphTemplateVertex::Free {
			return false;
		}
		let v1 = match self.vertices().filter(|&(_, &v_type)| v_type == g1).next() {
			Some((v1, _)) => { v1 }
			None => { return false; }
		};
		let v2 = match self.vertices().filter(|&(_, &v_type)| v_type == g2).next() {
			Some((v2, _)) => { v2 }
			None => { return false; }
		};
		let n1: HashSet<_> = self.neighbours(v1).collect();
		let n2: HashSet<_> = self.neighbours(v2).collect();
		n1 == n2
	}

	pub fn decompose(&self) -> Vec<Self> {
		let g: StableUnGraph<(), ()> = self.filter_map(
			|_, &vtype| if vtype == GraphTemplateVertex::Free { Some(()) } else { None },
			|_, _| Some(())
		).into();
		let mut parts = vec![];
		let mut scc = petgraph::algo::TarjanScc::new();
		scc.run(&g, |comp: &[NodeIndex]| {
			let comp_nodes = self.vertices()
				.filter(|&(_, &v_type)| v_type != GraphTemplateVertex::Free)
				.map(|(v, _)| v)
				.filter(|&v| self.neighbours(v).any(|u| comp.contains(&u)))
				.chain(comp.iter().copied())
				.collect::<HashSet<_>>();
			let graph = self.vertex_subgraph(|u, _| comp_nodes.contains(&u));
			parts.push(graph);
		});
		parts
	}

	pub fn apply(&self, g: &Graph, group_map: &HashMap<NodeIndex, GraphTemplateVertex>) -> Graph {
		let mut h = g.clone();
		let mut free: HashMap<NodeIndex, NodeIndex> = Default::default();
		// add vertices
		for (v, v_type) in self.vertices() {
			match v_type {
				GraphTemplateVertex::Free => {
					let u = h.add_vertex();
					free.insert(v, u);
				}
				GraphTemplateVertex::Group(_) => {}
			}
		}
		// add edges between free vertices
		for (u, v) in self.edges() {
			if self[u] != GraphTemplateVertex::Free || self[v] != GraphTemplateVertex::Free {
				continue;
			}
			let (u, v) = (free[&u], free[&v]);
			h.add_edge(u, v);
		}
		// find group vertices
		let group_rev_map = self.vertices()
			.map(|(v, &v_type)| (v_type, v))
			.filter(|&(v_type, _)| v_type != GraphTemplateVertex::Free)
			.collect::<HashMap<_, _>>();
		// add edges between free vertices and groups
		for (&u, &u_type) in group_map {
			if u_type == GraphTemplateVertex::Free {
				continue;
			}
			let u_group = group_rev_map[&u_type];
			for v_free in self.neighbours(u_group) {
				// we assume all neighbours of group vertices are free vertices
				let v = free[&v_free];
				h.add_edge(u, v);
			}
		}
		return h;
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphTemplateLimits {
	pub n_vertices: usize,
	pub n_groups: usize,

	pub permute: bool,
	pub no_free_components: bool,
	pub no_vertex_multiplication: bool
}

impl CombGrad<GraphTemplateLimits> for GraphTemplate {
	fn degree(&self) -> GraphTemplateLimits {
		GraphTemplateLimits {
			n_vertices: self.num_verts(),
			n_groups: self.groups().len(),
			// the rest does not matter much
			permute: true,
			no_free_components: false,
			no_vertex_multiplication: false
		}
	}
}
impl CombEnum<GraphTemplateLimits> for GraphTemplate {
	type Iter = Box<dyn Iterator<Item=Self> + Send + Sync>;
	fn iterate_deg_inner(limits: GraphTemplateLimits) -> Self::Iter {
		Box::new(Graph::iterate_deg(limits.n_vertices)
			.flat_map(move |g| {
				let mut gr_iter: Box<dyn Iterator<Item=Vec<NodeIndex>> + Send + Sync> = Box::new(g.vertices()
					.map(|(v, _)| v)
					.collect::<Vec<_>>()
					.into_iter()
					.combinations(limits.n_groups)
					.filter({
						// no edges between group vertices
						let g = g.clone();
						move |gr_verts| {
							gr_verts.iter()
								.tuple_combinations()
								.all(|(&u, &v)| !g.has_edge(u, v))
						}
					})
				);
				if limits.no_free_components {
					gr_iter = Box::new(gr_iter.filter({
						// no free connected components
						let g = g.clone();
						move |gr_verts| {
							let mut scc = petgraph::algo::TarjanScc::new();
							let mut good = true;
							scc.run(&g.0, |comp: &[NodeIndex]| {
								if comp.iter().all(|v| !gr_verts.contains(v)) {
									good = false;
								}
							});
							good
						}
					}));
				}
				if limits.no_vertex_multiplication {
					gr_iter = Box::new(gr_iter.filter({
						// vertex multiplication
						let g = g.clone();
						move |gr_verts| {
							// old indices change after removing a vertex, but it isn't difficult to work around that
							let last_u = g.vertices().map(|(v, _)| v).max_by_key(|u| *u).unwrap();
							for (v, _) in g.vertices() {
								if gr_verts.contains(&v) {
									continue;
								}
								let mut h = g.clone();
								h.delete_vertex(v);
								let mut scc = petgraph::algo::TarjanScc::new();
								let mut good = true;
								scc.run(&h.0, |comp: &[NodeIndex]| {
									// recall that `last_u` now actually occupies the index of `v`
									// if both are the same, we just never encounter the index
									if comp.iter().all(|u| if u == &v { !gr_verts.contains(&last_u) } else { !gr_verts.contains(u) }) {
										good = false;
									}
								});
								if !good {
									return false;
								}
							}
							return true;
						}
					}));
				}
				let template_iter = gr_iter.flat_map(move |gr_verts| {
					if limits.permute {
						gr_verts.into_iter()
							.permutations(limits.n_groups)
							.map(move |gr_verts| {
								gr_verts.into_iter()
									.enumerate()
									.fold(vec![GraphTemplateVertex::Free; limits.n_vertices], |mut acc, (gr_i, v)| {
										acc[v.index()] = GraphTemplateVertex::Group(gr_i as u8);
										acc
									})
							})
							.map({
								let g = g.clone();
								move |mapping| {
									GraphTemplate::new(g.clone(), mapping.as_slice())
								}
							})
							.collect::<Vec<_>>()
							.into_iter()
					} else {
						let mapping = gr_verts.into_iter()
							.enumerate()
							.fold(vec![GraphTemplateVertex::Free; limits.n_vertices], |mut acc, (gr_i, v)| {
								acc[v.index()] = GraphTemplateVertex::Group(gr_i as u8);
								acc
							});
						vec![GraphTemplate::new(g.clone(), mapping.as_slice())].into_iter()
					}
				});
				#[cfg(feature = "rayon")]
				let set = template_iter.par_bridge().collect::<CombSet<_>>();
				#[cfg(not(feature = "rayon"))]
				let set = template_iter.collect::<CombSet<_>>();
				set
			})
		)
	}
}
