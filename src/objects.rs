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
	fn count_deg(degree: T) -> Option<usize> {
		Self::iterate_deg(degree).size_hint().1
	}
}

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
#[cfg(feature = "petgraph")]
pub mod graphs;

pub mod chord_diagram;
