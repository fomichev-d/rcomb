use crate::*;
use crate::objects::chord_diagram::*;
use crate::objects::graph::*;
use crate::collections::set::*;

use std::fmt::Display;
use std::hash::Hash;

const OEIS_A156809: &[usize] = &[
	1, 1, 2, 4, 11, 34, 154, 978, 9497, 127954,
	2165291, 42609994, 937233306, 22576188846
];

#[derive(Clone, Debug)]
pub struct IntersectionGraph {
	graph: Graph,
	diagram: ChordDiagram
}
impl PartialEq for IntersectionGraph {
	fn eq(&self, other: &Self) -> bool {
	    self.diagram.eq(&other.diagram)
	}
}
impl Eq for IntersectionGraph {}
impl Hash for IntersectionGraph {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
	    self.diagram.hash(state);
	}
}
impl Display for IntersectionGraph {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}->{}", self.diagram, self.graph)
	}
}
impl From<ChordDiagram> for IntersectionGraph {
	fn from(diagram: ChordDiagram) -> Self {
	    let graph = diagram.intersection_graph();
		Self { graph, diagram }
	}
}
impl From<IntersectionGraph> for ChordDiagram {
	fn from(int_graph: IntersectionGraph) -> Self {
	    int_graph.diagram
	}
}
impl From<IntersectionGraph> for Graph {
	fn from(int_graph: IntersectionGraph) -> Self {
	    int_graph.graph
	}
}
impl CombGrad<usize> for IntersectionGraph {
	fn degree(&self) -> usize { self.graph.degree() }
}
impl CombEnum<usize> for IntersectionGraph {
	type Iter = Box<dyn Iterator<Item=Self> + Send + Sync>;
	fn iterate_deg_inner(degree: usize) -> Self::Iter {
		Box::new(ChordDiagram::iterate_deg(degree)
			.map(|diag| IntersectionGraph::from(diag))
			.filter({
				let mut graph_set = CombSet::<_>::new();
				move |int_graph| {
					#[cfg(feature = "rayon")]
					if graph_set.par_contains(&int_graph.graph) {
						false
					} else {
						graph_set.insert_unchecked(int_graph.graph.clone());
						true
					}
					#[cfg(not(feature = "rayon"))]
					if graph_set.contains(&int_graph.graph) {
						false
					} else {
						graph_set.insert_unchecked(int_graph.graph.clone());
						true
					}
				}
			})
		)
	}
	fn count_deg(degree: usize) -> Option<usize> {
		OEIS_A156809.get(degree).copied()
	}
}
impl IntersectionGraph {
	pub fn into_pair(self) -> (Graph, ChordDiagram) {
		(self.graph, self.diagram)
	}
	pub fn graph(&self) -> &Graph { &self.graph }
	pub fn into_graph(self) -> Graph { self.graph }
	pub fn diagram(&self) -> &ChordDiagram { &self.diagram }
	pub fn into_diagram(self) -> ChordDiagram { self.diagram }
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_iterator_count() {
		for n in 0..=7 {
			assert_eq!(IntersectionGraph::iterate_deg(n).count(), OEIS_A156809[n]);
		}
	}
}
