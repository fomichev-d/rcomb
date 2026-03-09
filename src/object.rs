pub trait CombEq {
	fn hash(&self) -> Vec<usize>;
	fn is_isomorphic(&self, other: &Self) -> bool;
}

pub trait Grading<T: Copy + Eq + Ord + Send + Sync = usize> {
	fn degree(&self) -> T;
}

pub trait CombOfDegree<T: Copy + Eq + Ord + Send + Sync>: Grading<T> {
	type Iter: Iterator<Item=Self>;
	fn of_degree(degree: T) -> Self::Iter;
}
