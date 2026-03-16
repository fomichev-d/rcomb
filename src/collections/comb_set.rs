use crate::*;
use crate::collections::*;
use crate::io::csv::*;

#[cfg(feature = "rayon")]
use rayon::iter::{
	ParallelIterator,
	IntoParallelIterator,
	FromParallelIterator,
	ParallelExtend
};

/// A set structure where item equality is considered up to isomorphism.
///
/// It should be used when checking key isomorphism is significantly more computationally expensive than computing a hash.
/// The hash [must be invariant under isomorphism](CombEq).
///
/// If key equality can be quickly computed, use [`HashSet`](std::collections::HashSet) instead.
/// This includes [CombCan] objects.
///
/// The set will normally contain at most one entry for each item isomorphism class.
/// For performance reasons, it is possible to temporarily violate this property by using [`insert_unchecked`](Self::insert_unchecked) or [`extend_unchecked`](Self::extend_unchecked).
/// Use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup) after these methods to restore the guarantees unless it is known they indeed were not violated.
#[derive(Debug)]
pub struct CombSet<G: CombEq, S: CombStats<G> = ()>(CombMap<G, (), S>);

impl<G: CombEq, S: CombStats<G>> Default for CombSet<G, S> {
	fn default() -> Self { Self(Default::default()) }
}
impl<G: CombEq, S: CombStats<G>> HasStats<G> for CombSet<G, S> {
	type Stats = S;
	#[inline]
	fn stats(&self) -> &Self::Stats { self.0.stats() }
}
impl<G: CombEq + Clone, S: CombStats<G>> Clone for CombSet<G, S> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}
impl<G: CombEq, S: CombStats<G>> FromIterator<G> for CombSet<G, S> {
	fn from_iter<I: IntoIterator<Item=G>>(it: I) -> Self {
		let mut set = Self::new();
		set.extend_unchecked(it);
		set
	}
}
impl<G: CombEq, S: CombStats<G>> IntoIterator for CombSet<G, S> {
	type IntoIter = std::iter::Map<<CombMap::<G, (), S> as IntoIterator>::IntoIter, fn((G, ())) -> G>;
	type Item = G;
	fn into_iter(self) -> Self::IntoIter {
		self.0.into_iter().map(entry_key as fn((G, ())) -> G)
	}
}
impl<'a, G: CombEq, S: CombStats<G>> IntoIterator for &'a CombSet<G, S> {
	type IntoIter = std::iter::Map<<&'a CombMap<G, (), S> as IntoIterator>::IntoIter, fn((&'a G, &'a ())) -> &'a G>;
	type Item = &'a G;
	fn into_iter(self) -> Self::IntoIter {
		(&self.0).into_iter().map(entry_key as fn((&'a G, &'a ())) -> &'a G)
	}
}
impl<'a, G: CombEq, S: CombStats<G>> IntoIterator for &'a mut CombSet<G, S> {
	type IntoIter = std::iter::Map<<&'a mut CombMap<G, (), S> as IntoIterator>::IntoIter, fn((&'a G, &'a mut ())) -> &'a G>;
	type Item = &'a G;
	fn into_iter(self) -> Self::IntoIter {
		(&mut self.0).into_iter().map(entry_key as fn((&'a G, &'a mut ())) -> &'a G)
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, S: CombStats<G>> FromParallelIterator<G> for CombSet<G, S> {
	fn from_par_iter<I: IntoParallelIterator<Item=G>>(par_iter: I) -> Self {
		Self(CombMap::<G, (), S>::from_par_iter(par_iter.into_par_iter().map(|g| (g, ()))))
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, S: CombStats<G>> ParallelExtend<G> for CombSet<G, S> {
	fn par_extend<I: IntoParallelIterator<Item=G>>(&mut self, par_iter: I) {
		self.0.par_extend(par_iter.into_par_iter().map(|g| (g, ())))
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send, S: CombStats<G>> IntoParallelIterator for CombSet<G, S> {
	type Iter = rayon::iter::Map<<CombMap::<G, (), S> as IntoParallelIterator>::Iter, fn((G, ())) -> G>;
	type Item = G;
	fn into_par_iter(self) -> Self::Iter {
		self.0.into_par_iter().map(entry_key as fn((G, ())) -> G)
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<'a, G: CombEq + Sync, S: CombStats<G>> IntoParallelIterator for &'a CombSet<G, S> {
	type Iter = rayon::iter::Map<<&'a CombMap<G, (), S> as IntoParallelIterator>::Iter, fn((&'a G, &'a ())) -> &'a G>;
	type Item = &'a G;
	fn into_par_iter(self) -> Self::Iter {
		(&self.0).into_par_iter().map(entry_key as fn((&'a G, &'a ())) -> &'a G)
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<'a, G: CombEq + Send + Sync, S: CombStats<G>> IntoParallelIterator for &'a mut CombSet<G, S> {
	type Iter = rayon::iter::Map<<&'a mut CombMap<G, (), S> as IntoParallelIterator>::Iter, fn((&'a G, &'a mut ())) -> &'a G>;
	type Item = &'a G;
	fn into_par_iter(self) -> Self::Iter {
		(&mut self.0).into_par_iter().map(entry_key as fn((&'a G, &'a mut ())) -> &'a G)
	}
}
impl<G: CombEq, S: CombStats<G>> Extend<G> for CombSet<G, S> {
	#[inline]
	fn extend<I>(&mut self, it: I) where I: IntoIterator<Item=G> {
		self.0.extend(it.into_iter().map(|g| (g, ())))
	}
}
impl<G: CombEq, S: CombStats<G>> CombSet<G, S> {
	/// Creates an empty set.
	pub fn new() -> Self { Self::default() }
	/// Clears the set, removing all entries.
	#[inline]
	pub fn clear(&mut self) { self.0.clear() }
	/// Returns the number of elements in the set.
	///
	/// If there are several isomorphic items (e.g. after [`insert_unchecked`](Self::insert_unchecked)), they will be counted separately.
	/// To restore item uniqueness, use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup).
	#[inline]
	pub fn len(&self) -> usize { self.0.len() }
	/// Inserts an item into the set.
	///
	/// If the set did have an isomorphic item present, it will not be replaced; this matters for items that can be isomorphic without being identical.
	///
	/// If there are several isomorphic items (e.g. after [`insert_unchecked`](Self::insert_unchecked)), an arbitrary one is picked to be replaced.
	/// To restore key uniqueness, use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup).
	#[inline]
	pub fn insert(&mut self, g: G) { self.0.insert(g, ()); }
	/// Inserts an item into the set, assuming it is not isomorphic to any present ones.
	///
	/// If the set did have an isomorphic item present, it will now store several isomorphic items.
	/// To restore key uniqueness, use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup).
	#[inline]
	pub fn insert_unchecked(&mut self, g: G) { self.0.insert_unchecked(g, ()) }
	/// Removes an item from the set.
	///
	/// If there are several isomorphic items (e.g. after [`insert_unchecked`](Self::insert_unchecked)), an arbitrary one is picked.
	/// To restore key uniqueness, use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup).
	#[inline]
	pub fn remove(&mut self, g: &G) { self.0.remove(g); }
	/// Extends the set with the contents of the iterator, assuming the items are not isomorphic to each other or any present ones.
	///
	/// If the set or the iterator did have isomorphic items, it will now store several isomorphic items.
	/// To restore key uniqueness, use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup).
	#[inline]
	pub fn extend_unchecked<I: IntoIterator<Item=G>>(&mut self, it: I) { self.0.extend_unchecked(it.into_iter().map(|g| (g, ()))) }
	/// Retains only the elements specified by the predicate.
	///
	/// In other words, remove items `g` for which `f(&g)` returns `false`.
	/// The elements are visited in unsorted (and unspecified) order.
	#[inline]
	pub fn retain<F: Fn(&G) -> bool + Copy>(&mut self, f: F) { self.0.retain(|g, ()| f(g)) }
	/// Remove duplicate items (up to isomorphism).
	/// The choice of the remaining item is arbitrary.
	#[inline]
	pub fn dedup(&mut self) { self.0.dedup() }
	/// Returns `true` if the set contains an isomorphic item.
	#[inline]
	pub fn contains(&self, g: &G) -> bool { self.0.contains_key(g) }
	/// An iterator visiting all items in arbitrary order.
	/// The iterator element type is `&'a G`.
	#[inline]
	pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
		self.into_iter()
	}
	#[inline]
	pub fn read_csv(config: CsvConfig<G, ()>) -> std::io::Result<Self> where G: CombCsv {
		Ok(Self(CombMap::read_csv(config)?))
	}
	#[inline]
	pub fn save_csv(&self, config: CsvConfig<G, ()>) -> std::io::Result<()> where G: CombCsv {
		self.0.save_csv(config)
	}
}
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, S: CombStats<G>> CombSet<G, S> {
	#[inline]
	pub fn par_insert(&mut self, g: G) {
		self.0.par_insert(g, ());
	}
	#[inline]
	pub fn par_remove(&mut self, g: &G) {
		self.0.par_remove(g);
	}
	#[inline]
	pub fn par_extend_unchecked<I: IntoParallelIterator<Item=G>>(&mut self, par_iter: I) {
		self.0.par_extend_unchecked(par_iter.into_par_iter().map(|g| (g, ())));
	}
	#[inline]
	pub fn par_retain<F: Fn(&G) -> bool + Copy + Sync>(&mut self, f: F) { self.0.par_retain(|g, ()| f(g)) }
	#[inline]
	pub fn par_contains(&self, g: &G) -> bool { self.0.par_contains_key(g) }
	#[inline]
	pub fn par_dedup(&mut self) { self.0.par_dedup() }
	#[inline]
	pub fn par_contains_key(&self, g: &G) -> bool { self.0.par_contains_key(g) }
	#[inline]
	pub fn par_read_csv(config: CsvConfig<G, ()>) -> std::io::Result<Self> where G: CombCsv {
		Ok(Self(CombMap::par_read_csv(config)?))
	}
	#[inline]
	pub fn save_ord_csv(&self, config: CsvConfig<G, ()>) -> std::io::Result<()> where G: CombCsv + CombEnum<usize>, G::Iter: Send + Sync {
		self.0.save_ord_csv(config)
	}
}
