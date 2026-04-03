use crate::objects::graph::*;

use std::{collections::{HashMap, HashSet}, fmt::Display, str::FromStr};

use petgraph::{graph::NodeIndex, prelude::StableUnGraph};
use itertools::Itertools;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum GraphTemplateVertex {
	Free,
	Group(u8)
}
impl NodeMatch for GraphTemplateVertex {}

type GraphTemplate = Graph<GraphTemplateVertex>;

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
		write!(f, "{{{} [", self.num_verts())?;
		for (i, e) in self.edges().enumerate() {
			if i > 0 {
				write!(f, ", ")?;
			}
			format_edge(self, e, f)?;
		}
		write!(f, "]")?;
		write!(f, "}}")
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
