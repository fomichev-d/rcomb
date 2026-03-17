#![cfg_attr(docsrs, feature(doc_cfg))]

#![allow(clippy::needless_range_loop)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::manual_is_multiple_of)]

#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
pub use rayon;
#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
#[cfg(feature = "petgraph")]
pub use petgraph;

// modules

mod util;
pub(crate) use util::*;

pub mod collections {
	pub mod map;
	pub mod set;
	pub mod index;
}

pub mod objects {
	#[cfg(feature = "petgraph")]
	mod graphs;
	#[cfg(feature = "petgraph")]
	pub use graphs::*;

	mod chord_diagram;
	pub use chord_diagram::*;
}

pub mod io {
	mod csv;
	pub use csv::*;
}

// core traits

pub trait CombEq {
	fn hash(&self) -> Vec<usize>;
	fn is_isomorphic(&self, other: &Self) -> bool;
}

pub trait CombGrad<T: Copy + Eq + Ord + Send + Sync = usize> {
	fn degree(&self) -> T;
}

pub trait CombEnum<T: Copy + Eq + Ord + Send + Sync>: CombGrad<T> {
	type Iter: Iterator<Item=Self>;
	fn iterate_deg(degree: T) -> Self::Iter;
	fn count_deg(degree: T) -> Option<usize> {
		Self::iterate_deg(degree).size_hint().1
	}
}

pub trait CombCan: Sized + Eq {
	type Input;
	#[allow(unused_variables)]
	fn validate(input: &Self::Input) -> bool { true }
	fn canonicalise(input: &mut Self::Input);
	unsafe fn from_raw(input: Self::Input) -> Self;

	fn new_unchecked(input: Self::Input) -> Self {
		assert!(Self::validate(&input));
		unsafe { Self::from_raw(input) }
	}
	fn new(mut input: Self::Input) -> Self {
		assert!(Self::validate(&input));
		Self::canonicalise(&mut input);
		unsafe { Self::from_raw(input) }
	}
}
