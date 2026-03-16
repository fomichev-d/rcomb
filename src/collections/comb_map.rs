use crate::*;
use crate::collections::*;
use crate::io::csv::*;
use std::fmt::Debug;

#[cfg(feature = "rayon")]
use rayon::iter::{
	ParallelBridge,
	FromParallelIterator,
	IndexedParallelIterator,
	IntoParallelIterator,
	IntoParallelRefIterator,
	IntoParallelRefMutIterator,
	ParallelIterator,
	ParallelExtend
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
impl<T: Copy + Eq + Ord + Ord + Send + Sync + Default, G: CombGrad<T>> CombStats<G> for MaxDegree<T> {
	#[inline]
	fn on_insert(&mut self, item: &G) {
		self.max_deg = std::cmp::max(self.max_deg, item.degree());
	}
	#[inline]
	fn clear(&mut self) {
		self.max_deg = Default::default();
	}
}

// TODO: map isomorphism

/// A map structure where key equality is considered up to isomorphism.
///
/// It should be used when checking key isomorphism is significantly more computationally expensive than computing a hash.
/// The hash [must be invariant under isomorphism](CombEq).
///
/// If key equality can be quickly computed, use [`HashMap`](std::collections::HashMap) instead.
/// This includes [CombCan] objects.
///
/// The map will normally contain at most one entry for each key isomorphism class.
/// For performance reasons, it is possible to temporarily violate this property by using [`insert_unchecked`](Self::insert_unchecked) or [`extend_unchecked`](Self::extend_unchecked).
/// Use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup) after these methods to restore the guarantees unless it is known they indeed were not violated.
pub struct CombMap<G: CombEq, T, S: CombStats<G> = ()> {
	buckets: HashMap<Vec<usize>, Vec<(G, T)>>,
	stats: S
}
impl<G: CombEq, T, S: CombStats<G>> Default for CombMap<G, T, S> {
	#[inline]
	fn default() -> Self { Self { buckets: HashMap::new(), stats: S::default() } }
}
impl<G: CombEq + Debug, T: Debug, S: CombStats<G>> Debug for CombMap<G, T, S> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.debug_map().entries(self.iter()).finish()
	}
}
impl<G: CombEq, T, S: CombStats<G>> HasStats<G> for CombMap<G, T, S> {
	type Stats = S;
	fn stats(&self) -> &Self::Stats { &self.stats }
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
impl<G: CombEq, T, S: CombStats<G>> FromIterator<(G, T)> for CombMap<G, T, S> {
	fn from_iter<I: IntoIterator<Item = (G, T)>>(it: I) -> Self {
		let mut map = Self::new();
		map.extend(it);
		map
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
impl<G: CombEq, T, S: CombStats<G>> IntoIterator for CombMap<G, T, S> {
	type IntoIter = std::iter::Flatten<std::collections::hash_map::IntoValues<Vec<usize>, Vec<(G, T)>>>;
	type Item = (G, T);
	fn into_iter(self) -> Self::IntoIter {
		self.buckets.into_values().flatten()
	}
}
impl<'a, G: CombEq, T, S: CombStats<G>> IntoIterator for &'a CombMap<G, T, S> {
	type IntoIter = std::iter::Map<std::iter::Flatten<std::collections::hash_map::Values<'a, Vec<usize>, Vec<(G, T)>>>, fn(&'a (G, T)) -> (&'a G, &'a T)>;
	type Item = (&'a G, &'a T);
	fn into_iter(self) -> Self::IntoIter {
		self.buckets.values().flatten().map(move_refs as fn(&'a (G, T)) -> (&'a G, &'a T))
	}
}
impl<'a, G: CombEq, T, S: CombStats<G>> IntoIterator for &'a mut CombMap<G, T, S> {
	type IntoIter = std::iter::Map<std::iter::Flatten<std::collections::hash_map::ValuesMut<'a, Vec<usize>, Vec<(G, T)>>>, fn(&'a mut (G, T)) -> (&'a G, &'a mut T)>;
	type Item = (&'a G, &'a mut T);
	fn into_iter(self) -> Self::IntoIter {
		self.buckets.values_mut().flatten().map(move_refs_mut2 as fn(&'a mut (G, T)) -> (&'a G, &'a mut T))
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> FromParallelIterator<(G, T)> for CombMap<G, T, S> {
	fn from_par_iter<I: IntoParallelIterator<Item=(G, T)>>(par_iter: I) -> Self {
		let mut buckets = par_iter.into_par_iter()
			.map(|(g, val)| (g.hash(), g, val))
			.fold(
				|| HashMap::<Vec<usize>, Vec<(G, T)>>::new(),
				|mut map, (key, g, val)| {
					map.entry(key).or_default().push((g, val));
					map
				}
			)
			.reduce(
				|| HashMap::<Vec<usize>, Vec<(G, T)>>::new(),
				|mut map1, map2| {
					for (key, mut bucket) in map2 {
						map1.entry(key).or_default().append(&mut bucket);
					}
					map1
				}
			);
		buckets.values_mut()
			.par_bridge()
			.for_each(|bucket| {
				par_dedup(bucket, |(g1, _), (g2, _)| g1.is_isomorphic(g2))
			});
		let mut stats = S::default();
		buckets.values()
			.flat_map(|bucket| bucket)
			.for_each(|(g, _)| { stats.on_insert(g); });
		Self { buckets, stats }
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> ParallelExtend<(G, T)> for CombMap<G, T, S> {
	fn par_extend<I: IntoParallelIterator<Item=(G, T)>>(&mut self, par_iter: I) {
		let buckets = par_iter.into_par_iter()
			.map(|(g, val)| (g.hash(), g, val))
			.fold(
				|| HashMap::<Vec<usize>, Vec<(G, T)>>::new(),
				|mut map, (key, g, val)| {
					map.entry(key).or_default().push((g, val));
					map
				}
			)
			.reduce(
				|| HashMap::<Vec<usize>, Vec<(G, T)>>::new(),
				|mut map1, map2| {
					for (key, mut bucket) in map2 {
						map1.entry(key).or_default().append(&mut bucket);
					}
					map1
				}
			);
		buckets.values()
			.flat_map(|bucket| bucket)
			.for_each(|(g, _)| { self.stats.on_insert(g); });
		for (key, mut bucket) in buckets {
			self.buckets.entry(key).or_default().append(&mut bucket);
		}
		self.buckets.values_mut()
			.par_bridge()
			.for_each(|bucket| {
				par_dedup(bucket, |(g1, _), (g2, _)| g1.is_isomorphic(g2))
			});
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send, T: Send, S: CombStats<G>> IntoParallelIterator for CombMap<G, T, S> {
	type Iter = rayon::iter::FlatMap<rayon::collections::hash_map::IntoIter<Vec<usize>, Vec<(G, T)>>, fn((Vec<usize>, Vec<(G, T)>)) -> Vec<(G, T)>>;
	type Item = (G, T);
	fn into_par_iter(self) -> Self::Iter {
		self.buckets.into_par_iter().flat_map(entry_value as fn((Vec<usize>, Vec<(G, T)>)) -> Vec<(G, T)>)
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<'a, G: CombEq + Sync, T: Sync, S: CombStats<G>> IntoParallelIterator for &'a CombMap<G, T, S> {
	type Iter = rayon::iter::Map<rayon::iter::FlatMap<rayon::collections::hash_map::Iter<'a, Vec<usize>, Vec<(G, T)>>, fn((&'a Vec<usize>, &'a Vec<(G, T)>)) -> &'a Vec<(G, T)>>, fn(&'a (G, T)) -> (&'a G, &'a T)>;
	type Item = (&'a G, &'a T);
	fn into_par_iter(self) -> Self::Iter {
		self.buckets.par_iter()
			.flat_map(entry_value as fn((&'a Vec<usize>, &'a Vec<(G, T)>)) -> &'a Vec<(G, T)>)
			.map(move_refs as fn(&'a (G, T)) -> (&'a G, &'a T))
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<'a, G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> IntoParallelIterator for &'a mut CombMap<G, T, S> {
	type Iter = rayon::iter::Map<rayon::iter::FlatMap<rayon::collections::hash_map::IterMut<'a, Vec<usize>, Vec<(G, T)>>, fn((&'a Vec<usize>, &'a mut Vec<(G, T)>)) -> &'a mut Vec<(G, T)>>, fn(&'a mut (G, T)) -> (&'a G, &'a mut T)>;
	type Item = (&'a G, &'a mut T);
	fn into_par_iter(self) -> Self::Iter {
		self.buckets.par_iter_mut()
			.flat_map(entry_value as fn((&'a Vec<usize>, &'a mut Vec<(G, T)>)) -> &'a mut Vec<(G, T)>)
			.map(move_refs_mut2 as fn(&'a mut (G, T)) -> (&'a G, &'a mut T))
	}
}
// we have two options (get() and par_get()) and no specialisation
/*
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
*/
impl<G: CombEq, T, S: CombStats<G>> CombMap<G, T, S> {
	/// Creates an empty map.
	pub fn new() -> Self { Self::default() }
	/// Clears the map, removing all key-value pairs.
	pub fn clear(&mut self) {
		self.buckets.clear();
		self.stats.clear();
	}
	/// Returns the number of elements in the map.
	///
	/// If there are several entries with isomorphic keys (e.g. after [`insert_unchecked`](Self::insert_unchecked)), they will be counted separately.
	/// To restore key uniqueness, use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup).
	pub fn len(&self) -> usize {
		self.buckets.iter()
			.map(|(_, v)| v.len())
			.sum()
	}
	/// Inserts a key-value pair into the map.
	///
	/// If the map did not have this key present, `None` is returned.
	///
	/// If the map did have this key present, the value is updated and the old value is returned.
	/// The key is not updated though; this matters for keys that can be isomorphic without being identical.
	///
	/// If there are several entries with isomorphic keys (e.g. after [`insert_unchecked`](Self::insert_unchecked)), an arbitrary one is picked to be replaced.
	/// To restore key uniqueness, use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup).
	#[inline]
	pub fn insert(&mut self, g: G, val: T) -> Option<T> {
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
	/// Inserts a key-value pair into the map, assuming the key is not isomorphic to any present ones.
	///
	/// If the map did have an isomorphic key present, it will now store several entries with isomorphic keys.
	/// To restore key uniqueness, use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup).
	#[inline]
	pub fn insert_unchecked(&mut self, g: G, val: T) {
		let key = g.hash();
		if self.buckets.get(&key).is_none() {
			self.buckets.insert(key.clone(), vec![]);
		}
		let bucket = self.buckets.get_mut(&key).unwrap();
		self.stats.on_insert(&g);
		bucket.push((g, val));
	}
	/// Removes a key from the map, returning the value at the key if an isomorphic key was previously in the map.
	///
	/// If there are several entries with isomorphic keys (e.g. after [`insert_unchecked`](Self::insert_unchecked)), an arbitrary one is picked.
	/// To restore key uniqueness, use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup).
	#[inline]
	pub fn remove(&mut self, g: &G) -> Option<T> {
		let key = g.hash();
		match self.buckets.get_mut(&key) {
			Some(bucket) => {
				if let Some(i) = bucket.iter().position(|(g_other, _)| g_other.is_isomorphic(g)) {
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
	/// Extends the map with the contents of the iterator, assuming the keys are not isomorphic to each other or any present ones.
	///
	/// If the map or the iterator did have isomorphic keys, it will now store several entries with isomorphic keys.
	/// To restore key uniqueness, use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup).
	#[inline]
	pub fn extend_unchecked<I: IntoIterator<Item=(G, T)>>(&mut self, it: I) {
		for (g, val) in it {
			self.insert_unchecked(g, val);
		}
	}
	/// Retains only the elements specified by the predicate.
	///
	/// In other words, remove all pairs `(k, v)` for which `f(&k, &mut v)` returns `false`.
	/// The elements are visited in unsorted (and unspecified) order.
	#[inline]
	pub fn retain<F: Fn(&G, &mut T) -> bool>(&mut self, f: F) {
		self.buckets.iter_mut().for_each(|(_, bucket)| {
			bucket.retain_mut(|(g, v)| f(g, v));
		});
		self.buckets.retain(|_, v| { v.len() > 0 });
	}
	/// Remove entries with duplicate keys (up to isomorphism).
	/// The choice of the remaining key is arbitrary.
	pub fn dedup(&mut self) {
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
	/// Returns a reference to the value corresponding to the key.
	///
	/// If there are several entries with isomorphic keys (e.g. after [`insert_unchecked`](Self::insert_unchecked)), an arbitrary one is picked.
	/// To restore key uniqueness, use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup).
	#[inline]
	pub fn get(&self, g: &G) -> Option<&T> {
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
	/// Returns a mutable reference to the value corresponding to the key.
	///
	/// If there are several entries with isomorphic keys (e.g. after [`insert_unchecked`](Self::insert_unchecked)), an arbitrary one is picked.
	/// To restore key uniqueness, use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup).
	#[inline]
	pub fn get_mut(&mut self, g: &G) -> Option<&mut T> {
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
	/// Returns `true` if the map contains a value for the specified key.
	#[inline]
	pub fn contains_key(&self, g: &G) -> bool {
		self.get(g).is_some()
	}
	/// An iterator visiting all key-value pairs in arbitrary order.
	/// The iterator element type is `(&'a G, &'a T)`.
	#[inline]
	pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
		self.into_iter()
	}
	/// An iterator visiting all key-value pairs in arbitrary order, with mutable references to the values.
	/// The iterator element type is `(&'a G, &'a mut T)`.
	#[inline]
	pub fn iter_mut(&mut self) -> <&mut Self as IntoIterator>::IntoIter {
		self.into_iter()
	}
	/// Creates a consuming iterator visiting all keys in arbitrary order.
	/// The map cannot be used after calling this.
	/// The iterator element type is `G`.
	#[inline]
	pub fn into_keys(self) -> std::iter::Map<<Self as IntoIterator>::IntoIter, fn((G, T)) -> G> {
		self.into_iter().map(entry_key as fn((G, T)) -> G)
	}
	/// An iterator visiting all keys in arbitrary order.
	/// The iterator element type is `&'a G`.
	#[inline]
	pub fn keys<'a>(&'a self) -> std::iter::Map<<&'a Self as IntoIterator>::IntoIter, fn((&'a G, &'a T)) -> &'a G> {
		self.into_iter().map(entry_key as fn((&'a G, &'a T)) -> &'a G)
	}
	/// Creates a consuming iterator visiting all values in arbitrary order.
	/// The map cannot be used after calling this.
	/// The iterator element type is `T`.
	#[inline]
	pub fn into_values(self) -> std::iter::Map<<Self as IntoIterator>::IntoIter, fn((G, T)) -> T> {
		self.into_iter().map(entry_value as fn((G, T)) -> T)
	}
	/// An iterator visiting all values in arbitrary order.
	/// The iterator element type is `&'a T`.
	#[inline]
	pub fn values<'a>(&'a self) -> std::iter::Map<<&'a Self as IntoIterator>::IntoIter, fn((&'a G, &'a T)) -> &'a T> {
		self.into_iter().map(entry_value as fn((&'a G, &'a T)) -> &'a T)
	}
	/// An iterator visiting all values mutably in arbitrary order.
	/// The iterator element type is `&'a mut T`.
	#[inline]
	pub fn values_mut<'a>(&'a mut self) -> std::iter::Map<<&'a mut Self as IntoIterator>::IntoIter, fn((&'a G, &'a mut T)) -> &'a mut T> {
		self.into_iter().map(entry_value as fn((&'a G, &'a mut T)) -> &'a mut T)
	}

	pub fn efficiency(&self) -> f64 {
		let l = self.len();
		if l == 0 { return 1.; }
		return l as f64 / self.buckets.len() as f64;
	}
	pub fn apply<T2, F: Fn(T) -> T2>(self, f: F) -> CombMap<G, T2, S> {
		let buckets = self.buckets.into_iter().map(|(key, bucket)| (
			key.clone(),
			bucket.into_iter().map(|(g, val)| (g, f(val))).collect()
		)).collect();
		CombMap { buckets, stats: self.stats.clone() }
	}
	pub fn apply_ref<T2, F: Fn(&T) -> T2>(&self, f: F) -> CombMap<G, T2, S> where G: Clone {
		let buckets = self.buckets.iter().map(|(key, bucket)| (
			key.clone(),
			bucket.into_iter().map(|(g, val)| (g.clone(), f(val))).collect()
		)).collect();
		CombMap { buckets, stats: self.stats.clone() }
	}
	pub fn with_stats<S2: CombStats<G>>(self) -> CombMap<G, T, S2> where G: Sync, T: Sync {
		let mut stats = S2::default();
		self.keys().for_each(|g| stats.on_insert(g));
		CombMap { buckets: self.buckets, stats }
	}

	pub fn read_csv(config: CsvConfig<G, T>) -> std::io::Result<Self> where G: CombCsv {
		#[cfg(feature = "kdam")]
		use kdam::TqdmIterator;

		let reader = csv::ReaderBuilder::new()
			.has_headers(config.use_header)
			.from_path(&config.filename)?;
		let mut map = Self::new();

		#[allow(unused_mut)]
		let mut it: Box<dyn Iterator<Item=csv::StringRecord>> = Box::new(reader.into_records()
			.filter_map(|result| result.ok())
		);
		#[cfg(feature = "kdam")]
		if config.use_tqdm { it = Box::new(it.tqdm()); }

		for (g, val) in it.filter_map(|record| config.read_entry(&record)) {
			if let Some(val) = val {
				map.insert_unchecked(g, val);
			} else {
				if std::mem::size_of::<T>() == std::mem::size_of::<()>() {
					map.insert_unchecked(g, unsafe { std::mem::MaybeUninit::<T>::zeroed().assume_init() });
				} else {
					panic!("CsvConfig::parse_value() was not called for a non-empty type T!");
				}
			}
		}
		Ok(map)
	}
	pub fn save_csv(&self, config: CsvConfig<G, T>) -> std::io::Result<()> where G: CombCsv {
		#[cfg(feature = "kdam")]
		use kdam::TqdmIterator;

		let mut writer = csv::Writer::from_path(&config.filename)?;
		if config.use_header { writer.write_record(config.write_header())?; }

		#[allow(unused_mut)]
		let mut it: Box<dyn Iterator<Item=(&G, &T)>> = Box::new(self.iter());

		#[cfg(feature = "kdam")]
		if config.use_tqdm {
			it = Box::new(it.tqdm());
		}

		for (g, val) in it {
			if let Some(entry) = config.write_entry(g, val) {
				writer.write_record(entry)?;
			}
		}
		Ok(())
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, T: Send + Sync, S: CombStats<G>> CombMap<G, T, S> {
	#[inline]
	pub fn par_insert(&mut self, g: G, val: T) -> Option<T> {
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
	pub fn par_remove(&mut self, g: &G) -> Option<T> {
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
	pub fn par_extend_unchecked<I: IntoParallelIterator<Item=(G, T)>>(&mut self, par_iter: I) {
		let buckets = par_iter.into_par_iter()
			.map(|(g, val)| (g.hash(), g, val))
			.fold(
				|| HashMap::<Vec<usize>, Vec<(G, T)>>::new(),
				|mut map, (hash, g, val)| {
					map.entry(hash).or_default().push((g, val));
					map
				}
			)
			.reduce(
				|| HashMap::<Vec<usize>, Vec<(G, T)>>::new(),
				|mut map1, map2| {
					for (hash, mut bucket) in map2 {
						map1.entry(hash).or_default().append(&mut bucket);
					}
					map1
				}
			);
		buckets.values()
			.flat_map(|bucket| bucket)
			.for_each(|(g, _)| { self.stats.on_insert(g); });
		for (hash, mut bucket) in buckets {
			self.buckets.entry(hash).or_default().append(&mut bucket);
		}
	}
	pub fn par_retain<F: Fn(&G, &mut T) -> bool + Copy + Sync>(&mut self, f: F) {
		self.buckets.par_iter_mut().for_each(|(_, bucket)| {
			bucket.retain_mut(|(g, v)| f(g, v));
		});
		self.buckets.retain(|_, v| { v.len() > 0 });
	}
	pub fn par_dedup(&mut self) {
		self.buckets.par_iter_mut().for_each(|(_, bucket)| {
			par_dedup(bucket, |(g1, _), (g2, _)| g1.is_isomorphic(g2));
		});
	}
	#[inline]
	pub fn par_get(&self, g: &G) -> Option<&T> {
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
	pub fn par_get_mut(&mut self, g: &G) -> Option<&mut T> {
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
	pub fn par_contains_key(&self, g: &G) -> bool {
		self.par_get(g).is_some()
	}
	#[inline]
	pub fn into_par_keys(self) -> rayon::iter::Map<<Self as IntoParallelIterator>::Iter, fn((G, T)) -> G> {
		self.into_par_iter().map(entry_key as fn((G, T)) -> G)
	}
	#[inline]
	pub fn par_keys<'a>(&'a self) -> rayon::iter::Map<<&'a Self as IntoParallelIterator>::Iter, fn((&'a G, &'a T)) -> &'a G> {
		self.par_iter().map(entry_key as fn((&'a G, &'a T)) -> &'a G)
	}
	#[inline]
	pub fn into_par_values(self) -> rayon::iter::Map<<Self as IntoParallelIterator>::Iter, fn((G, T)) -> T> {
		self.into_par_iter().map(entry_value as fn((G, T)) -> T)
	}
	#[inline]
	pub fn par_values<'a>(&'a self) -> rayon::iter::Map<<&'a Self as IntoParallelIterator>::Iter, fn((&'a G, &'a T)) -> &'a T> {
		self.par_iter().map(entry_value as fn((&'a G, &'a T)) -> &'a T)
	}
	#[inline]
	pub fn par_values_mut<'a>(&'a mut self) -> rayon::iter::Map<<&'a mut Self as IntoParallelIterator>::Iter, fn((&'a G, &'a mut T)) -> &'a mut T> {
		self.par_iter_mut().map(entry_value as fn((&'a G, &'a mut T)) -> &'a mut T)
	}

	pub fn par_apply<T2: Send, F: Fn(T) -> T2 + Sync>(self, f: F) -> CombMap<G, T2, S> {
		let buckets = self.buckets.into_par_iter()
			.map(|(key, bucket)| (
				key.clone(),
				bucket.into_par_iter()
					.map(|(g, val)| (g, f(val)))
					.collect()
			))
			.collect();
		CombMap { buckets, stats: self.stats.clone() }
	}
	pub fn par_apply_ref<T2: Send, F: Fn(&T) -> T2 + Sync>(&self, f: F) -> CombMap<G, T2, S> where G: Clone {
		let buckets = self.buckets.par_iter()
			.map(|(key, bucket)| (
				key.clone(),
				bucket.into_par_iter()
					.map(|(g, val)| (g.clone(), f(val)))
					.collect()
			))
			.collect();
		CombMap { buckets, stats: self.stats.clone() }
	}

	pub fn par_read_csv(config: CsvConfig<G, T>) -> std::io::Result<Self> where G: CombCsv {
		#[cfg(feature = "kdam")]
		use kdam::TqdmIterator;

		let reader = csv::ReaderBuilder::new()
			.has_headers(config.use_header)
			.from_path(&config.filename)?;
		
		#[allow(unused_mut)]
		let mut it: Box<dyn Iterator<Item=csv::StringRecord> + Send + Sync> = Box::new(reader.into_records()
			.filter_map(|result| result.ok())
		);
		#[cfg(feature = "kdam")]
		if config.use_tqdm { it = Box::new(it.tqdm()); }

		let map = it.par_bridge()
			.filter_map(|record| config.read_entry(&record))
			.map(|(g, val)| {
				if let Some(val) = val {
					(g, val)
				} else {
					if std::mem::size_of::<T>() == std::mem::size_of::<()>() {
						(g, unsafe { std::mem::MaybeUninit::<T>::zeroed().assume_init() })
					} else {
						panic!("CsvConfig::parse_value() was not called for a non-empty type T!");
					}
				}
			})
			.collect::<Self>();
		Ok(map)
	}
	pub fn save_ord_csv(&self, config: CsvConfig<G, T>) -> std::io::Result<()> where G: CombCsv + CombEnum<usize>, G::Iter: Send + Sync {
		#[cfg(feature = "kdam")]
		use kdam::TqdmIterator;
		use rayon::prelude::ParallelSliceMut;

		let max_deg = self.keys()
			.par_bridge()
			.map(|g| g.degree())
			.max()
			.unwrap_or(0);

		let mut writer = csv::Writer::from_path(&config.filename)?;
		if config.use_header { writer.write_record(config.write_header())?; }

		for deg in 0..=max_deg {
			#[allow(unused_mut)]
			let mut it: Box<dyn Iterator<Item=(usize, G)> + Send + Sync> = Box::new(G::iterate_deg(deg).enumerate());
			#[cfg(feature = "kdam")]
			if config.use_tqdm { it = Box::new(it.tqdm()); }
			let mut entries = it.par_bridge()
				.filter_map(|(i, g)| {
					self.get(&g).map(|val| (i, g, val))
				})
				.filter_map(|(i, g, val)| config.write_entry(&g, val).map(|entry| (i, entry)))
				.collect::<Vec<_>>();
			entries.par_sort_unstable_by_key(|&(i, _)| i);
			for (_, entry) in entries {
				writer.write_record(entry)?;
			}
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::*;
	use super::*;
	impl CombEq for usize {
		fn hash(&self) -> Vec<usize> { vec![self % 2] }
		fn is_isomorphic(&self, other: &Self) -> bool { self == other }
	}
	impl CombGrad<usize> for usize {
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
		assert_eq!(map.get(&0), Some(&1));
		assert_eq!(map.get(&1), Some(&2));
		assert_eq!(map.get(&2), Some(&4));
		assert_eq!(map.insert(2, 3), Some(4));
		assert_eq!(map.get(&2), Some(&3));
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
