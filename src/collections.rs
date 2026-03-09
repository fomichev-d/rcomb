use crate::objects::*;

#[cfg(feature = "rayon")]
use rayon::iter::{
	ParallelIterator,
	IntoParallelIterator,
	IntoParallelRefIterator,
	IntoParallelRefMutIterator,
	IndexedParallelIterator
};

use std::collections::HashMap;

// stats 

pub trait HasStats<G: CombEq> {
	type Stats: CombStats<G>;
	fn stats(&self) -> &Self::Stats;
}

pub trait CombStats<T>: Clone + Default + Sync {
	fn on_insert(&mut self, item: &T);
	fn clear(&mut self);
}
impl<T> CombStats<T> for () {
	fn on_insert(&mut self, _: &T) {}
	fn clear(&mut self) {}
}

#[derive(Clone, Copy, Default, Debug)]
pub struct MaxDegree<T: Copy + Eq + Ord + Send + Sync + Default = usize> {
	pub max_deg: T
}
impl<T: Copy + Eq + Ord + Ord + Send + Sync + Default, G: Grading<T>> CombStats<G> for MaxDegree<T> {
	#[inline]
	fn on_insert(&mut self, item: &G) {
		self.max_deg = std::cmp::max(self.max_deg, item.degree());
	}
	#[inline]
	fn clear(&mut self) {
		self.max_deg = Default::default();
	}
}

// helpers

#[inline]
fn bucket_values<G, T>(tuple: (Vec<usize>, Vec<(G, T)>)) -> Vec<(G, T)> { tuple.1 }
#[inline]
fn bucket_values_ref<'a, G, T>(tuple: (&'a Vec<usize>, &'a Vec<(G, T)>)) -> &'a Vec<(G, T)> { tuple.1 }
#[inline]
fn bucket_values_ref_mut<'a, G, T>(tuple: (&'a Vec<usize>, &'a mut Vec<(G, T)>)) -> &'a mut Vec<(G, T)> { tuple.1 }
#[inline]
fn entry_key<G, T>(tuple: (G, T)) -> G { tuple.0 }
#[inline]
fn entry_key_ref<'a, G, T>(tuple: (&'a G, &'a T)) -> &'a G { &tuple.0 }
#[cfg(feature = "rayon")]
#[inline]
fn entry_key_ref_mut2<'a, G, T>(tuple: (&'a G, &'a mut T)) -> &'a G { &tuple.0 }
#[inline]
fn move_refs<'a, G, T>(tuple: &'a (G, T)) -> (&'a G, &'a T) { (&tuple.0, &tuple.1) }
#[inline]
fn move_refs_mut2<'a, G, T>(tuple: &'a mut (G, T)) -> (&'a G, &'a mut T) { (&tuple.0, &mut tuple.1) }

// map & set traits

/// A map trait where key equality is considered up to isomorphism.
///
/// It should be used when checking key isomorphism is significantly more computationally expensive than computing a hash.
/// The hash [must be invariant under isomorphism](`CombEq`).
///
/// The map will normally contain at most one entry for each key isomorphism class.
/// For performance reasons, it is possible to temporarily violate this property by using [`insert_unchecked`](`CombMapBase::insert_unchecked`) or [`extend_unchecked`](`CombMapBase::extend_unchecked`).
/// Use [`dedup`](`CompMapBase::dedup`) after these methods to restore the guarantees unless it is known they indeed were not violated.
pub trait CombMapBase<G: CombEq, T>: Default + Extend<(G, T)> {
	/// Creates an empty map.
	fn new() -> Self { Self::default() }
	/// Clears the map, removing all key-value pairs.
	fn clear(&mut self);
	/// Returns the number of elements in the map.
	///
	/// If there are several entries with isomorphic keys (e.g. after [`insert_unchecked`](`CompMapBase::insert_unchecked`)), they will be counted separately.
	/// To restore key uniqueness, use [`dedup`](`CompMapBase::dedup`).
	fn len(&self) -> usize;
	/// Inserts a key-value pair into the map.
	///
	/// If the map did not have this key present, `None` is returned.
	///
	/// If the map did have this key present, the value is updated and the old value is returned.
	/// The key is not updated though; this matters for keys that can be isomorphic without being identical.
	///
	/// If there are several entries with isomorphic keys (e.g. after [`insert_unchecked`](`CompMapBase::insert_unchecked`)), an arbitrary one is picked to be replaced.
	/// To restore key uniqueness, use [`dedup`](`CompMapBase::dedup`).
	fn insert(&mut self, g: G, val: T) -> Option<T>;
	/// Inserts a key-value pair into the map, assuming the key is not isomorphic to any present ones.
	///
	/// If the map did have an isomorphic key present, it will now store several entries with isomorphic keys.
	/// To restore key uniqueness, use [`dedup`](`CompMapBase::dedup`).
	fn insert_unchecked(&mut self, g: G, val: T);
	/// Removes a key from the map, returning the value at the key if an isomorphic key was previously in the map.
	///
	/// If there are several entries with isomorphic keys (e.g. after [`insert_unchecked`](`CompMapBase::insert_unchecked`)), an arbitrary one is picked.
	/// To restore key uniqueness, use [`dedup`](`CompMapBase::dedup`).
	fn remove(&mut self, g: &G) -> Option<T>;
	/// Extends the map with the contents of the iterator, assuming the keys are not isomorphic to each other or any isomorphic ones.
	///
	/// If the map or the iterator did have isomorphic keys, it will now store several entries with isomorphic keys.
	/// To restore key uniqueness, use [`dedup`](`CompMapBase::dedup`).
	fn extend_unchecked<I: IntoIterator<Item=(G, T)>>(&mut self, it: I);
	/// Retains only the elements specified by the predicate.
	///
	/// In other words, remove all pairs `(k, v)` for which `f(&k, &mut v)` returns `false`.
	/// The elements are visited in unsorted (and unspecified) order.
	fn retain<F: Fn(&G, &mut T) -> bool + Copy + Sync>(&mut self, f: F);
	/// Remove entries with duplicate keys (up to isomorphism).
	/// The choice of the remaining key is arbitrary.
	fn dedup(&mut self);
	/// Returns a reference to the value corresponding to the key.
	///
	/// If there are several entries with isomorphic keys (e.g. after [`insert_unchecked`](`CompMapBase::insert_unchecked`)), an arbitrary one is picked.
	/// To restore key uniqueness, use [`dedup`](`CompMapBase::dedup`).
	fn get(&self, g: &G) -> Option<&T>;
	/// Returns a mutable reference to the value corresponding to the key.
	///
	/// If there are several entries with isomorphic keys (e.g. after [`insert_unchecked`](`CompMapBase::insert_unchecked`)), an arbitrary one is picked.
	/// To restore key uniqueness, use [`dedup`](`CompMapBase::dedup`).
	fn get_mut(&mut self, g: &G) -> Option<&mut T>;
	/// Returns `true` if the map contains a value for the specified key.
	fn contains_key(&self, g: &G) -> bool {
		self.get(g).is_some()
	}
	/// An iterator visiting all key-value pairs in arbitrary order.
	/// The iterator element type is `(&'a G, &'a T)`.
	fn iter<'a>(&'a self) -> impl Iterator<Item=(&'a G, &'a T)> + Sync + Send where G: 'a + Sync, T: 'a + Sync;
	/// An iterator visiting all key-value pairs in arbitrary order, with mutable references to the values.
	/// The iterator element type is `(&'a G, &'a mut T)`.
	fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item=(&'a G, &'a mut T)> + Sync + Send where G: 'a + Sync + Send, T: 'a + Sync + Send;
	/// An iterator visiting all keys in arbitrary order. The iterator element type is `&'a G`.
	fn keys<'a>(&'a self) -> impl Iterator<Item=&'a G> + Sync + Send where G: 'a + Sync, T: 'a + Sync;
}

pub trait CombSetBase<G: CombEq>: Default + Extend<G> {
	fn new() -> Self { Self::default() }
	fn clear(&mut self);
	fn len(&self) -> usize;
	fn insert(&mut self, g: G);
	fn insert_unchecked(&mut self, g: G);
	fn remove(&mut self, g: &G);
	fn extend_unchecked<I: IntoIterator<Item=G>>(&mut self, it: I);
	fn retain<F: Fn(&G) -> bool + Copy + Sync>(&mut self, f: F);
	fn dedup(&mut self);
	fn contains(&self, g: &G) -> bool;

	fn iter<'a>(&'a self) -> impl Iterator<Item=&'a G> + Sync + Send where G: 'a + Sync;
}

// CombMap implementation

#[derive(Debug)]
pub struct CombMap<G: CombEq, T, S: CombStats<G> = ()> {
	buckets: HashMap<Vec<usize>, Vec<(G, T)>>,
	stats: S
}
impl<G: CombEq, T, S: CombStats<G>> HasStats<G> for CombMap<G, T, S> {
	type Stats = S;
	fn stats(&self) -> &Self::Stats { &self.stats }
}
impl<G: CombEq, T, S: CombStats<G>> Default for CombMap<G, T, S> {
	#[inline]
	fn default() -> Self { Self { buckets: HashMap::new(), stats: S::default() } }
}
impl<G: CombEq + Clone, T: Clone, S: CombStats<G>> Clone for CombMap<G, T, S> {
	fn clone(&self) -> Self {
		Self {
			buckets: self.buckets.clone(),
			stats: self.stats.clone()
		}
	}
}
impl<G: CombEq + Clone, T, S: CombStats<G>> CombMap<G, T, S> {
	pub fn clone_with<F: Fn(&T) -> T>(&self, clone: F) -> Self {
		Self {
			buckets: self.buckets.iter()
				.map(|(k, v)| (
					k.clone(),
					v.iter()
						.map(|(g, val)| (g.clone(), clone(val)))
						.collect()
				))
				.collect(),
			stats: self.stats.clone()
		}
	}
}
impl<G: CombEq, T, S: CombStats<G>> Extend<(G, T)> for CombMap<G, T, S> {
	#[inline]
	fn extend<I>(&mut self, it: I) where I: IntoIterator<Item=(G, T)> {
		for (g, val) in it {
			self.insert(g, val);
		}
	}
}
impl<G: CombEq, T, S: CombStats<G>> FromIterator<(G, T)> for CombMap<G, T, S> {
	fn from_iter<I: IntoIterator<Item = (G, T)>>(it: I) -> Self {
		let mut map = Self::new();
		map.extend(it);
		map
	}
}
impl<G: CombEq, T, S: CombStats<G>> IntoIterator for CombMap<G, T, S> {
	type IntoIter = std::iter::FlatMap<std::collections::hash_map::IntoIter<Vec<usize>, Vec<(G, T)>>, Vec<(G, T)>, fn((Vec<usize>, Vec<(G, T)>)) -> Vec<(G, T)>>;
	type Item = (G, T);
	fn into_iter(self) -> Self::IntoIter {
		self.buckets.into_iter().flat_map(bucket_values as fn((Vec<usize>, Vec<(G, T)>)) -> Vec<(G, T)>)
	}
}
impl<'a, G: CombEq, T, S: CombStats<G>> IntoIterator for &'a CombMap<G, T, S> {
	type IntoIter = std::iter::Map<std::iter::FlatMap<std::collections::hash_map::Iter<'a, Vec<usize>, Vec<(G, T)>>, &'a Vec<(G, T)>, fn((&'a Vec<usize>, &'a Vec<(G, T)>)) -> &'a Vec<(G, T)>>, fn(&'a (G, T)) -> (&'a G, &'a T)>;
	type Item = (&'a G, &'a T);
	fn into_iter(self) -> Self::IntoIter {
		self.buckets.iter()
			.flat_map(bucket_values_ref as fn((&'a Vec<usize>, &'a Vec<(G, T)>)) -> &'a Vec<(G, T)>)
			.map(move_refs as fn(&'a (G, T)) -> (&'a G, &'a T))
	}
}
impl<'a, G: CombEq, T, S: CombStats<G>> IntoIterator for &'a mut CombMap<G, T, S> {
	type IntoIter = std::iter::Map<std::iter::FlatMap<std::collections::hash_map::IterMut<'a, Vec<usize>, Vec<(G, T)>>, &'a mut Vec<(G, T)>, fn((&'a Vec<usize>, &'a mut Vec<(G, T)>)) -> &'a mut Vec<(G, T)>>, fn(&'a mut (G, T)) -> (&'a G, &'a mut T)>;
	type Item = (&'a G, &'a mut T);
	fn into_iter(self) -> Self::IntoIter {
		self.buckets.iter_mut()
			.flat_map(bucket_values_ref_mut as fn((&'a Vec<usize>, &'a mut Vec<(G, T)>)) -> &'a mut Vec<(G, T)>)
			.map(move_refs_mut2 as fn(&'a mut (G, T)) -> (&'a G, &'a mut T))
	}
}
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> IntoParallelIterator for CombMap<G, T, S> {
	type Iter = rayon::iter::FlatMap<rayon::collections::hash_map::IntoIter<Vec<usize>, Vec<(G, T)>>, fn((Vec<usize>, Vec<(G, T)>)) -> Vec<(G, T)>>;
	type Item = (G, T);
	fn into_par_iter(self) -> Self::Iter {
		self.buckets.into_par_iter().flat_map(bucket_values as fn((Vec<usize>, Vec<(G, T)>)) -> Vec<(G, T)>)
	}
}
#[cfg(feature = "rayon")]
impl<'a, G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> IntoParallelIterator for &'a CombMap<G, T, S> {
	type Iter = rayon::iter::Map<rayon::iter::FlatMap<rayon::collections::hash_map::Iter<'a, Vec<usize>, Vec<(G, T)>>, fn((&'a Vec<usize>, &'a Vec<(G, T)>)) -> &'a Vec<(G, T)>>, fn(&'a (G, T)) -> (&'a G, &'a T)>;
	type Item = (&'a G, &'a T);
	fn into_par_iter(self) -> Self::Iter {
		self.buckets.par_iter()
			.flat_map(bucket_values_ref as fn((&'a Vec<usize>, &'a Vec<(G, T)>)) -> &'a Vec<(G, T)>)
			.map(move_refs as fn(&'a (G, T)) -> (&'a G, &'a T))
	}
}
#[cfg(feature = "rayon")]
impl<'a, G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> IntoParallelIterator for &'a mut CombMap<G, T, S> {
	type Iter = rayon::iter::Map<rayon::iter::FlatMap<rayon::collections::hash_map::IterMut<'a, Vec<usize>, Vec<(G, T)>>, fn((&'a Vec<usize>, &'a mut Vec<(G, T)>)) -> &'a mut Vec<(G, T)>>, fn(&'a mut (G, T)) -> (&'a G, &'a mut T)>;
	type Item = (&'a G, &'a mut T);
	fn into_par_iter(self) -> Self::Iter {
		self.buckets.par_iter_mut()
			.flat_map(bucket_values_ref_mut as fn((&'a Vec<usize>, &'a mut Vec<(G, T)>)) -> &'a mut Vec<(G, T)>)
			.map(move_refs_mut2 as fn(&'a mut (G, T)) -> (&'a G, &'a mut T))
	}
}
impl<G: CombEq, T, S: CombStats<G>> std::ops::Index<&G> for CombMap<G, T, S> {
	type Output = T;
	#[inline]
	fn index(&self, key: &G) -> &Self::Output {
		self.get(key).expect("no entry found for key")
	}
}
impl<G: CombEq, T, S: CombStats<G>> std::ops::IndexMut<&G> for CombMap<G, T, S> {
	#[inline]
	fn index_mut(&mut self, key: &G) -> &mut Self::Output {
		self.get_mut(key).expect("no entry found for key")
	}
}
impl<G: CombEq, T, S: CombStats<G>> CombMapBase<G, T> for CombMap<G, T, S> {
	fn clear(&mut self) {
		self.buckets.clear();
		self.stats.clear();
	}
	fn len(&self) -> usize {
		self.buckets.iter()
			.map(|(_, v)| v.len())
			.sum()
	}
	#[inline]
	fn insert(&mut self, g: G, val: T) -> Option<T> {
		let key = g.hash();
		if self.buckets.get(&key).is_none() {
			self.buckets.insert(key.clone(), vec![]);
		}
		let bucket = self.buckets.get_mut(&key).unwrap();
		match bucket.iter().position(|(g_other, _)| g.is_isomorphic(g_other)) {
			Some(i) => {
				Some(std::mem::replace(&mut bucket[i].1, val))
			}
			None => {
				self.stats.on_insert(&g);
				bucket.push((g, val));
				None
			}
		}
	}
	#[inline]
	fn insert_unchecked(&mut self, g: G, val: T) {
		let key = g.hash();
		if self.buckets.get(&key).is_none() {
			self.buckets.insert(key.clone(), vec![]);
		}
		let bucket = self.buckets.get_mut(&key).unwrap();
		self.stats.on_insert(&g);
		bucket.push((g, val));
	}
	#[inline]
	fn remove(&mut self, g: &G) -> Option<T> {
		let key = g.hash();
		match self.buckets.get_mut(&key) {
			Some(bucket) => {
				if let Some(i) = bucket.iter().position(|(g_other, _)| g_other.is_isomorphic(g)) {
					let (_, val) = bucket.remove(i);
					if bucket.len() == 0 {
						self.buckets.remove(&key);
					}
					Some(val)
				} else {
					None
				}
			}
			None => None
		}
	}
	#[inline]
	fn extend_unchecked<I: IntoIterator<Item=(G, T)>>(&mut self, it: I) {
		for (g, val) in it {
			self.insert_unchecked(g, val);
		}
	}
	#[inline]
	fn retain<F: Fn(&G, &mut T) -> bool + Copy + Sync>(&mut self, f: F) {
		self.buckets.iter_mut().for_each(|(_, bucket)| {
			bucket.retain_mut(|(g, v)| f(g, v));
		});
		self.buckets.retain(|_, v| { v.len() > 0 });
	}
	fn dedup(&mut self) {
		self.buckets.iter_mut().for_each(|(_, vals)| {
			for i in (0..vals.len()).rev() {
				for j in (i+1..vals.len()).rev() {
					if vals[i].0.is_isomorphic(&vals[j].0) {
						vals.remove(j);
					}
				}
			}
		});
	}
	#[inline]
	fn get(&self, g: &G) -> Option<&T> {
		let key = g.hash();
		match self.buckets.get(&key) {
			Some(bucket) => {
				for (g_other, val) in bucket.iter() {
					if g_other.is_isomorphic(g) {
						return Some(val);
					}
				}
				None
			}
			None => { None }
		}
	}
	#[inline]
	fn get_mut(&mut self, g: &G) -> Option<&mut T> {
		let key = g.hash();
		match self.buckets.get_mut(&key) {
			Some(bucket) => {
				for (g_other, val) in bucket.iter_mut() {
					if g_other.is_isomorphic(g) {
						return Some(val);
					}
				}
				None
			}
			None => { None }
		}
	}
	#[inline]
	fn iter<'a>(&'a self) -> impl Iterator<Item=(&'a G, &'a T)> + Sync + Send where G: 'a + Sync, T: 'a + Sync {
		self.into_iter()
	}
	#[inline]
	fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item=(&'a G, &'a mut T)> + Sync + Send where T: 'a + Sync + Send, G: 'a + Sync + Send {
		self.into_iter()
	}
	#[inline]
	fn keys<'a>(&'a self) -> impl Iterator<Item=&'a G> + Sync + Send where G: 'a + Sync, T: 'a + Sync {
		self.into_iter().map(|(g, _)| g)
	}
}
impl<G: CombEq, T, S: CombStats<G>> CombMap<G, T, S> {
	pub fn efficiency(&self) -> f64 {
		let l = self.len();
		if l == 0 { return 1.; }
		return l as f64 / self.buckets.len() as f64;
	}
	pub fn transform<T2, F: Fn(&T) -> T2>(&self, f: F) -> CombMap<G, T2, S> where G: Clone {
		let buckets = self.buckets.iter().map(|(k, a)| (
			k.clone(),
			a.into_iter().map(|(g, v)| (g.clone(), f(v))).collect()
		)).collect();
		CombMap { buckets, stats: self.stats.clone() }
	}
	pub fn with_stats<S2: CombStats<G>>(self) -> CombMap<G, T, S2> where G: Sync, T: Sync {
		let mut stats = S2::default();
		self.keys().for_each(|g| stats.on_insert(g));
		CombMap { buckets: self.buckets, stats }
	}
	#[cfg(feature = "rayon")]
	pub fn par(self) -> CombParMap<G, T, S> where G: Send + Sync, T: Send + Sync {
		CombParMap { buckets: self.buckets, stats: self.stats }
	}
}

// CombParMap implementation

#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
#[derive(Debug)]
pub struct CombParMap<G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G> = ()> {
	buckets: HashMap<Vec<usize>, Vec<(G, T)>>,
	stats: S
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> HasStats<G> for CombParMap<G, T, S> {
	type Stats = S;
	fn stats(&self) -> &Self::Stats { &self.stats }
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> Default for CombParMap<G, T, S> {
	fn default() -> Self { Self { buckets: HashMap::new(), stats: S::default() } }
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync + Clone, T: Send + Sync + Clone, S: CombStats<G>> Clone for CombParMap<G, T, S> {
	fn clone(&self) -> Self {
		Self {
			buckets: self.buckets.clone(),
			stats: self.stats.clone()
		}
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync + Clone, T: Send + Sync, S: CombStats<G>> CombParMap<G, T, S> {
	pub fn clone_with<F: Fn(&T) -> T>(&self, clone: F) -> Self {
		Self {
			buckets: self.buckets.iter()
				.map(|(k, v)| (
					k.clone(),
					v.iter()
						.map(|(g, val)| (g.clone(), clone(val)))
						.collect()
				))
				.collect(),
			stats: self.stats.clone()
		}
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> FromIterator<(G, T)> for CombParMap<G, T, S> {
	fn from_iter<I: IntoIterator<Item = (G, T)>>(it: I) -> Self {
		let mut map = Self::new();
		map.extend(it);
		map
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> IntoIterator for CombParMap<G, T, S> {
	type IntoIter = std::iter::FlatMap<std::collections::hash_map::IntoIter<Vec<usize>, Vec<(G, T)>>, Vec<(G, T)>, fn((Vec<usize>, Vec<(G, T)>)) -> Vec<(G, T)>>;
	type Item = (G, T);
	fn into_iter(self) -> Self::IntoIter {
		self.buckets.into_iter().flat_map(bucket_values as fn((Vec<usize>, Vec<(G, T)>)) -> Vec<(G, T)>)
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<'a, G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> IntoIterator for &'a CombParMap<G, T, S> {
	type IntoIter = std::iter::Map<std::iter::FlatMap<std::collections::hash_map::Iter<'a, Vec<usize>, Vec<(G, T)>>, &'a Vec<(G, T)>, fn((&'a Vec<usize>, &'a Vec<(G, T)>)) -> &'a Vec<(G, T)>>, fn(&'a (G, T)) -> (&'a G, &'a T)>;
	type Item = (&'a G, &'a T);
	fn into_iter(self) -> Self::IntoIter {
		self.buckets.iter()
			.flat_map(bucket_values_ref as fn((&'a Vec<usize>, &'a Vec<(G, T)>)) -> &'a Vec<(G, T)>)
			.map(move_refs as fn(&'a (G, T)) -> (&'a G, &'a T))
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<'a, G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> IntoIterator for &'a mut CombParMap<G, T, S> {
	type IntoIter = std::iter::Map<std::iter::FlatMap<std::collections::hash_map::IterMut<'a, Vec<usize>, Vec<(G, T)>>, &'a mut Vec<(G, T)>, fn((&'a Vec<usize>, &'a mut Vec<(G, T)>)) -> &'a mut Vec<(G, T)>>, fn(&'a mut (G, T)) -> (&'a G, &'a mut T)>;
	type Item = (&'a G, &'a mut T);
	fn into_iter(self) -> Self::IntoIter {
		self.buckets.iter_mut()
			.flat_map(bucket_values_ref_mut as fn((&'a Vec<usize>, &'a mut Vec<(G, T)>)) -> &'a mut Vec<(G, T)>)
			.map(move_refs_mut2 as fn(&'a mut (G, T)) -> (&'a G, &'a mut T))
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> IntoParallelIterator for CombParMap<G, T, S> {
	type Iter = rayon::iter::FlatMap<rayon::collections::hash_map::IntoIter<Vec<usize>, Vec<(G, T)>>, fn((Vec<usize>, Vec<(G, T)>)) -> Vec<(G, T)>>;
	type Item = (G, T);
	fn into_par_iter(self) -> Self::Iter {
		self.buckets.into_par_iter().flat_map(bucket_values as fn((Vec<usize>, Vec<(G, T)>)) -> Vec<(G, T)>)
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<'a, G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> IntoParallelIterator for &'a CombParMap<G, T, S> {
	type Iter = rayon::iter::Map<rayon::iter::FlatMap<rayon::collections::hash_map::Iter<'a, Vec<usize>, Vec<(G, T)>>, fn((&'a Vec<usize>, &'a Vec<(G, T)>)) -> &'a Vec<(G, T)>>, fn(&'a (G, T)) -> (&'a G, &'a T)>;
	type Item = (&'a G, &'a T);
	fn into_par_iter(self) -> Self::Iter {
		self.buckets.par_iter()
			.flat_map(bucket_values_ref as fn((&'a Vec<usize>, &'a Vec<(G, T)>)) -> &'a Vec<(G, T)>)
			.map(move_refs as fn(&'a (G, T)) -> (&'a G, &'a T))
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<'a, G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> IntoParallelIterator for &'a mut CombParMap<G, T, S> {
	type Iter = rayon::iter::Map<rayon::iter::FlatMap<rayon::collections::hash_map::IterMut<'a, Vec<usize>, Vec<(G, T)>>, fn((&'a Vec<usize>, &'a mut Vec<(G, T)>)) -> &'a mut Vec<(G, T)>>, fn(&'a mut (G, T)) -> (&'a G, &'a mut T)>;
	type Item = (&'a G, &'a mut T);
	fn into_par_iter(self) -> Self::Iter {
		self.buckets.par_iter_mut()
			.flat_map(bucket_values_ref_mut as fn((&'a Vec<usize>, &'a mut Vec<(G, T)>)) -> &'a mut Vec<(G, T)>)
			.map(move_refs_mut2 as fn(&'a mut (G, T)) -> (&'a G, &'a mut T))
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> std::ops::Index<&G> for CombParMap<G, T, S> {
	type Output = T;
	#[inline]
	fn index(&self, key: &G) -> &Self::Output {
		self.get(key).expect("no entry found for key")
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> std::ops::IndexMut<&G> for CombParMap<G, T, S> {
	#[inline]
	fn index_mut(&mut self, key: &G) -> &mut Self::Output {
		self.get_mut(key).expect("no entry found for key")
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> Extend<(G, T)> for CombParMap<G, T, S> {
	#[inline]
	fn extend<I>(&mut self, it: I) where I: IntoIterator<Item=(G, T)> {
		for (g, val) in it {
			self.insert(g, val);
		}
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> CombMapBase<G, T> for CombParMap<G, T, S> {
	fn clear(&mut self) {
		self.buckets.clear();
		self.stats.clear();
	}
	fn len(&self) -> usize {
		self.buckets.par_iter()
			.map(|(_, v)| v.len())
			.sum()
	}
	#[inline]
	fn insert(&mut self, g: G, val: T) -> Option<T> {
		let key = g.hash();
		if self.buckets.get(&key).is_none() {
			self.buckets.insert(key.clone(), vec![]);
		}
		let bucket = self.buckets.get_mut(&key).unwrap();
		match bucket.par_iter().position_any(|(g_other, _)| g.is_isomorphic(g_other)) {
			Some(i) => {
				Some(std::mem::replace(&mut bucket[i].1, val))
			}
			None => {
				self.stats.on_insert(&g);
				bucket.push((g, val));
				None
			}
		}
	}
	#[inline]
	fn insert_unchecked(&mut self, g: G, val: T) {
		let key = g.hash();
		if self.buckets.get(&key).is_none() {
			self.buckets.insert(key.clone(), vec![]);
		}
		let bucket = self.buckets.get_mut(&key).unwrap();
		self.stats.on_insert(&g);
		bucket.push((g, val));
	}
	#[inline]
	fn remove(&mut self, g: &G) -> Option<T> {
		let key = g.hash();
		match self.buckets.get_mut(&key) {
			Some(bucket) => {
				if let Some(i) = bucket.par_iter().position_any(|(g_other, _)| g_other.is_isomorphic(g)) {
					let (_, val) = bucket.swap_remove(i);
					if bucket.len() == 0 {
						self.buckets.remove(&key);
					}
					Some(val)
				} else {
					None
				}
			}
			None => None
		}
	}
	// TODO: sort out `I` bounds and make it truly parallel
	#[inline]
	fn extend_unchecked<I: IntoIterator<Item=(G, T)>>(&mut self, it: I) {
		for (g, val) in it {
			self.insert_unchecked(g, val);
		}
	}
	#[inline]
	fn retain<F: Fn(&G, &mut T) -> bool + Copy + Sync>(&mut self, f: F) {
		self.buckets.par_iter_mut().for_each(|(_, bucket)| {
			bucket.retain_mut(|(g, v)| f(g, v));
		});
		self.buckets.retain(|_, v| { v.len() > 0 });
	}
	fn dedup(&mut self) {
		self.buckets.par_iter_mut().for_each(|(_, vals)| {
			for i in (0..vals.len()).rev() {
				for j in (i+1..vals.len()).rev() {
					if vals[i].0.is_isomorphic(&vals[j].0) {
						vals.remove(j);
					}
				}
			}
		});
	}
	#[inline]
	fn get(&self, g: &G) -> Option<&T> {
		let key = g.hash();
		match self.buckets.get(&key) {
			Some(bucket) => {
				bucket.par_iter()
					.find_any(|(g_other, _)| g_other.is_isomorphic(g))
					.map(|(_, val)| val)
			}
			None => { None }
		}
	}
	#[inline]
	fn get_mut(&mut self, g: &G) -> Option<&mut T> {
		let key = g.hash();
		match self.buckets.get_mut(&key) {
			Some(bucket) => {
				bucket.par_iter_mut()
					.find_any(|(g_other, _)| g_other.is_isomorphic(g))
					.map(|(_, val)| val)
			}
			None => { None }
		}
	}
	#[inline]
	fn iter<'a>(&'a self) -> impl Iterator<Item=(&'a G, &'a T)> + Sync + Send where G: 'a + Sync, T: 'a + Sync {
		self.into_iter()
	}
	#[inline]
	fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item=(&'a G, &'a mut T)> + Sync + Send where G: 'a + Sync + Send, T: 'a + Sync + Send {
		self.into_iter()
	}
	#[inline]
	fn keys<'a>(&'a self) -> impl Iterator<Item=&'a G> + Sync + Send where G: 'a + Sync, T: 'a + Sync {
		self.into_iter().map(|(g, _)| g)
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> CombParMap<G, T, S> {
	pub fn efficiency(&self) -> f64 {
		let l = self.len();
		if l == 0 { return 1.; }
		return l as f64 / self.buckets.len() as f64;
	}
	pub fn transform<T2: Send + Sync, F: Fn(&T) -> T2 + Sync>(&self, f: F) -> CombParMap<G, T2, S> where G: Clone {
		let buckets = self.buckets.par_iter()
			.map(|(k, a)| (k.clone(), a.into_par_iter().map(|(g, v)| (g.clone(), f(v))).collect()))
			.collect();
		CombParMap { buckets, stats: self.stats.clone() }
	}
	pub fn with_stats<S2: CombStats<G>>(self) -> CombParMap<G, T, S2> {
		let mut stats = S2::default();
		self.keys().for_each(|g| stats.on_insert(g));
		CombParMap { buckets: self.buckets, stats }
	}
	pub fn seq(self) -> CombMap<G, T, S> {
		CombMap { buckets: self.buckets, stats: self.stats }
	}
}

// set implementation

#[derive(Debug)]
pub struct CombSetImpl<G: CombEq, M: CombMapBase<G, ()>>(M, std::marker::PhantomData<G>);
pub type CombSet<G, S = ()> = CombSetImpl<G, CombMap<G, (), S>>;
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
pub type CombParSet<G, S = ()> = CombSetImpl<G, CombParMap<G, (), S>>;
impl<G: CombEq, M: CombMapBase<G, ()>> Default for CombSetImpl<G, M> {
	fn default() -> Self { Self(Default::default(), Default::default()) }
}
impl<G: CombEq, M: CombMapBase<G, ()> + HasStats<G>> HasStats<G> for CombSetImpl<G, M> {
	type Stats = M::Stats;
	#[inline]
	fn stats(&self) -> &Self::Stats { self.0.stats() }
}
impl<G: CombEq, M: CombMapBase<G, ()> + Clone> Clone for CombSetImpl<G, M> {
	fn clone(&self) -> Self {
		Self(self.0.clone(), self.1.clone())
	}
}
impl<G: CombEq, M: CombMapBase<G, ()> + IntoIterator<Item=(G, ())>> IntoIterator for CombSetImpl<G, M> {
	type IntoIter = std::iter::Map<M::IntoIter, fn((G, ())) -> G>;
	type Item = G;
	fn into_iter(self) -> Self::IntoIter {
		self.0.into_iter().map(entry_key as fn((G, ())) -> G)
	}
}
impl<'a, G: CombEq, M: CombMapBase<G, ()>> IntoIterator for &'a CombSetImpl<G, M> where &'a M: IntoIterator<Item=(&'a G, &'a ())> {
	type IntoIter = std::iter::Map<<&'a M as IntoIterator>::IntoIter, fn((&'a G, &'a ())) -> &'a G>;
	type Item = &'a G;
	fn into_iter(self) -> Self::IntoIter {
		self.0.into_iter().map(entry_key_ref as fn((&'a G, &'a ())) -> &'a G)
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send, M: CombMapBase<G, ()> + IntoParallelIterator<Item=(G, ())>> IntoParallelIterator for CombSetImpl<G, M> {
	type Iter = rayon::iter::Map<M::Iter, fn((G, ())) -> G>;
	type Item = G;
	fn into_par_iter(self) -> Self::Iter {
		self.0.into_par_iter().map(entry_key as fn((G, ())) -> G)
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<'a, G: CombEq + Sync, M: CombMapBase<G, ()>> IntoParallelIterator for &'a CombSetImpl<G, M> where &'a M: IntoParallelIterator<Item=(&'a G, &'a ())> {
	type Iter = rayon::iter::Map<<&'a M as IntoParallelIterator>::Iter, fn((&'a G, &'a ())) -> &'a G>;
	type Item = &'a G;
	fn into_par_iter(self) -> Self::Iter {
		self.0.into_par_iter().map(entry_key_ref as fn((&'a G, &'a ())) -> &'a G)
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<'a, G: CombEq + Sync, M: CombMapBase<G, ()>> IntoParallelIterator for &'a mut CombSetImpl<G, M> where &'a mut M: IntoParallelIterator<Item=(&'a G, &'a mut ())> {
	type Iter = rayon::iter::Map<<&'a mut M as IntoParallelIterator>::Iter, fn((&'a G, &'a mut ())) -> &'a G>;
	type Item = &'a G;
	fn into_par_iter(self) -> Self::Iter {
		self.0.into_par_iter().map(entry_key_ref_mut2 as fn((&'a G, &'a mut ())) -> &'a G)
	}
}
impl<G: CombEq, M: CombMapBase<G, ()>> Extend<G> for CombSetImpl<G, M> {
	#[inline]
	fn extend<I>(&mut self, it: I) where I: IntoIterator<Item=G> {
		self.0.extend(it.into_iter().map(|g| (g, ())))
	}
}
impl<G: CombEq, M: CombMapBase<G, ()>> CombSetBase<G> for CombSetImpl<G, M> {
	#[inline]
	fn len(&self) -> usize { self.0.len() }
	#[inline]
	fn clear(&mut self) { self.0.clear() }
	#[inline]
	fn insert(&mut self, g: G) { self.0.insert(g, ()); }
	#[inline]
	fn insert_unchecked(&mut self, g: G) { self.0.insert_unchecked(g, ()) }
	#[inline]
	fn remove(&mut self, g: &G) { self.0.remove(g); }
	#[inline]
	fn extend_unchecked<I: IntoIterator<Item=G>>(&mut self, it: I) { self.0.extend_unchecked(it.into_iter().map(|g| (g, ()))) }
	#[inline]
	fn retain<F: Fn(&G) -> bool + Copy + Sync>(&mut self, f: F) { self.0.retain(|g, ()| f(g)) }
	#[inline]
	fn dedup(&mut self) { self.0.dedup() }
	#[inline]
	fn contains(&self, g: &G) -> bool { self.0.contains_key(g) }

	#[inline]
	fn iter<'a>(&'a self) -> impl Iterator<Item=&'a G> + Sync + Send where G: 'a + Sync { self.0.keys() }
}

#[cfg(test)]
mod tests {
	use crate::objects::*;
	use crate::collections::*;
	impl CombEq for usize {
		fn hash(&self) -> Vec<usize> { vec![self % 2] }
		fn is_isomorphic(&self, other: &Self) -> bool { self == other }
	}
	impl Grading<usize> for usize {
		fn degree(&self) -> usize { *self }
	}
	#[test]
	fn test_map() {
		let mut map = CombMap::<usize, usize>::new();
		assert_eq!(map.len(), 0);
		assert!(map.insert(0, 1).is_none());
		assert_eq!(map.len(), 1);
		assert!(map.insert(1, 2).is_none());
		assert_eq!(map.len(), 2);
		assert!(map.insert(2, 4).is_none());
		assert_eq!(map.len(), 3);
		assert_eq!(map[&0], 1);
		assert_eq!(map[&1], 2);
		assert_eq!(map[&2], 4);
		assert_eq!(map.insert(2, 3), Some(4));
		assert_eq!(map[&2], 3);
		assert_eq!(map.len(), 3);
		map.clear();
		assert_eq!(map.len(), 0);
		for i in 0..9 { map.insert(i, i % 3); }
		assert_eq!(map.len(), 9);
		map.retain(|_, v| *v == 0);
		assert_eq!(map.len(), 3);
		let mut keys: Vec<usize> = map.keys().copied().collect();
		keys.sort();
		assert_eq!(keys, vec![0, 3, 6]);
		map.extend(keys.iter().map(|&i| (i + 1, 1)));
		assert_eq!(map.len(), 6);
		map.insert_unchecked(0, 111);
		assert_eq!(map.len(), 7);
		map.extend_unchecked(keys.iter().map(|&i| (i + 1, 222)));
		assert_eq!(map.len(), 10);
		assert_eq!(map.remove(&6), Some(0));
		assert_eq!(map.len(), 9);
		map.dedup();
		assert_eq!(map.len(), 5);
		assert_eq!(map.contains_key(&6), false);
	}
}
