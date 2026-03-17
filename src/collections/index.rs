use crate::*;
use crate::collections::map::*;
#[cfg(feature = "rayon")]
use crate::collections::set::CombSet;

#[cfg(feature = "rayon")]
use rayon::iter::{
	FromParallelIterator,
	IntoParallelIterator,
	IntoParallelRefIterator,
	ParallelExtend,
	ParallelIterator
};

use std::borrow::Borrow;
use std::collections::HashMap;
#[cfg(feature = "rayon")]
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

// common traits

trait IndexExt<G> {
	fn next_index(&self) -> usize;
	fn set_next_index(&mut self, idx: usize);
	fn reassign(&mut self, src: usize, dst: usize);
	#[cfg(feature = "rayon")]
	fn par_reassign(&mut self, src: usize, dst: usize) where G: Send + Sync;
}

#[allow(private_bounds)]
pub trait IndexStrategy: Sized + Sync + Sealed {
	fn on_remove<G, Index: IndexExt<G>>(index: &mut Index, i: usize);
	#[cfg(feature = "rayon")]
	fn on_par_remove<G: Send + Sync, Index: IndexExt<G>>(index: &mut Index, i: usize);
}

#[derive(Clone, Copy, Debug)]
pub struct KeepIndex;
impl Sealed for KeepIndex {}
impl IndexStrategy for KeepIndex {
	#[allow(private_bounds)]
	fn on_remove<G, Index: IndexExt<G>>(_index: &mut Index, _i: usize) {}
	#[allow(private_bounds)]
	#[cfg(feature = "rayon")]
	fn on_par_remove<G: Send + Sync, Index: IndexExt<G>>(_index: &mut Index, _i: usize) {}
}

#[derive(Clone, Copy, Debug)]
pub struct ReuseIndex;
impl Sealed for ReuseIndex {}
impl IndexStrategy for ReuseIndex {
	#[allow(private_bounds)]
	fn on_remove<G, Index: IndexExt<G>>(index: &mut Index, i: usize) {
		let j = index.next_index() - 1;
		if i < j {
			index.reassign(j, i);
			index.set_next_index(j);
		}
	}
	#[allow(private_bounds)]
	#[cfg(feature = "rayon")]
	fn on_par_remove<G: Send + Sync, Index: IndexExt<G>>(index: &mut Index, i: usize) {
		let j = index.next_index() - 1;
		if i < j {
			index.par_reassign(j, i);
			index.set_next_index(j);
		}
	}
}

// CombIndex

#[allow(private_bounds)]
#[derive(Clone)]
pub struct CombIndex<G: CombEq, Strategy: IndexStrategy = KeepIndex> {
	keys: CombMap<G, usize>,
	vals: HashMap<usize, G>,
	next: usize,
	strategy: PhantomData<Strategy>
}
impl<G: CombEq, Strategy: IndexStrategy> IndexExt<G> for CombIndex<G, Strategy> {
	fn next_index(&self) -> usize {
	    self.next
	}
	fn set_next_index(&mut self, idx: usize) {
	    self.next = idx;
	}
	fn reassign(&mut self, src: usize, dst: usize) {
		let h = self.vals.remove(&src).unwrap();
		*self.keys.get_mut(&h).unwrap() = dst;
		self.vals.insert(dst, h);
	}
	#[cfg(feature = "rayon")]
	fn par_reassign(&mut self, src: usize, dst: usize) where G: Send + Sync {
		let h = self.vals.remove(&src).unwrap();
		*self.keys.par_get_mut(&h).unwrap() = dst;
		self.vals.insert(dst, h);
	}
}
impl<G: CombEq, Strategy: IndexStrategy> Default for CombIndex<G, Strategy> {
	fn default() -> Self {
		Self { keys: CombMap::new(), vals: HashMap::new(), next: 0, strategy: Default::default() }
	}
}
impl<G: CombEq + Debug, Strategy: IndexStrategy> Debug for CombIndex<G, Strategy> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.debug_map().entries(self.vals.iter()).finish()
	}
}
impl<G: CombEq + Clone, Strategy: IndexStrategy> FromIterator<G> for CombIndex<G, Strategy> {
	fn from_iter<I: IntoIterator<Item=G>>(it: I) -> Self {
		let mut index = Self::new();
		index.extend(it);
		index
	}
}
impl<G: CombEq + Clone, Strategy: IndexStrategy> Extend<G> for CombIndex<G, Strategy> {
	fn extend<I: IntoIterator<Item=G>>(&mut self, it: I) {
		for g in it {
			self.insert(g);
		}
	}
}
impl<G: CombEq, Strategy: IndexStrategy> IntoIterator for CombIndex<G, Strategy> {
	type IntoIter = std::collections::hash_map::IntoIter<usize, G>;
	type Item = (usize, G);
	fn into_iter(self) -> Self::IntoIter {
		self.vals.into_iter()
	}
}
impl<'a, G: CombEq, Strategy: IndexStrategy> IntoIterator for &'a CombIndex<G, Strategy> {
	type IntoIter = std::iter::Map<std::collections::hash_map::Iter<'a, usize, G>, fn((&'a usize, &'a G)) -> (usize, &'a G)>;
	type Item = (usize, &'a G);
	fn into_iter(self) -> Self::IntoIter {
		self.vals.iter().map(|(&i, g)| (i, g))
	}
}
impl<'a, G: CombEq, Strategy: IndexStrategy> IntoIterator for &'a mut CombIndex<G, Strategy> {
	type IntoIter = std::iter::Map<std::collections::hash_map::Iter<'a, usize, G>, fn((&'a usize, &'a G)) -> (usize, &'a G)>;
	type Item = (usize, &'a G);
	fn into_iter(self) -> Self::IntoIter {
		self.vals.iter().map(|(&i, g)| (i, g))
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Clone + Send + Sync, Strategy: IndexStrategy> FromParallelIterator<G> for CombIndex<G, Strategy> {
	fn from_par_iter<I: IntoParallelIterator<Item=G>>(par_iter: I) -> Self {
		let set: CombSet<G> = par_iter.into_par_iter().collect();
		let mut next = 0;
		let keys: CombMap<G, usize> = set.into_iter()
			.map(|g| {
				let i = next;
				next += 1;
				(g, i)
			})
			.collect();
		let vals: HashMap<usize, G> = keys.iter()
			.map(|(g, &i)| (i, g.clone()))
			.collect();
		Self { keys, vals, next, strategy: Default::default() }
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Clone + Send + Sync, Strategy: IndexStrategy> ParallelExtend<G> for CombIndex<G, Strategy> {
	fn par_extend<I: IntoParallelIterator<Item=G>>(&mut self, par_iter: I) {
		let set: CombSet<G> = par_iter.into_par_iter()
			.filter(|g| !self.par_contains_val(g))
			.collect();
		let n = set.len();
		if self.next.checked_add(n).is_none() {
			panic!("CombIndex can hold at most {} items!", usize::MAX - 1);
		}
		let mut next = self.next;
		let keys: CombMap<G, usize> = set.into_iter()
			.map(|g| {
				let i = next;
				next += 1;
				(g, i)
			})
			.collect();
		let vals: HashMap<usize, G> = keys.iter()
			.map(|(g, &i)| (i, g.clone()))
			.collect();
		self.keys.par_extend_unchecked(keys);
		self.vals.extend(vals);
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send, Strategy: IndexStrategy> IntoParallelIterator for CombIndex<G, Strategy> {
	type Iter = rayon::collections::hash_map::IntoIter<usize, G>;
	type Item = (usize, G);
	fn into_par_iter(self) -> Self::Iter {
		self.vals.into_par_iter()
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<'a, G: CombEq + Sync, Strategy: IndexStrategy> IntoParallelIterator for &'a CombIndex<G, Strategy> {
	type Iter = rayon::iter::Map<rayon::collections::hash_map::Iter<'a, usize, G>, fn((&'a usize, &'a G)) -> (usize, &'a G)>;
	type Item = (usize, &'a G);
	fn into_par_iter(self) -> Self::Iter {
		self.vals.par_iter().map(|(&i, g)| (i, g))
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<'a, G: CombEq + Sync, Strategy: IndexStrategy> IntoParallelIterator for &'a mut CombIndex<G, Strategy> {
	type Iter = rayon::iter::Map<rayon::collections::hash_map::Iter<'a, usize, G>, fn((&'a usize, &'a G)) -> (usize, &'a G)>;
	type Item = (usize, &'a G);
	fn into_par_iter(self) -> Self::Iter {
		self.vals.par_iter().map(|(&i, g)| (i, g))
	}
}
impl<G: CombEq + Clone, Strategy: IndexStrategy> std::ops::Index<usize> for CombIndex<G, Strategy> {
	type Output = G;
	#[inline]
	fn index(&self, key: usize) -> &Self::Output {
		self.val(key).expect("no value found for index")
	}
}
#[allow(private_bounds)]
impl<G: CombEq + Clone, Strategy: IndexStrategy> CombIndex<G, Strategy> {
	pub fn new() -> Self {
		Self::default()
	}
	pub fn len(&self) -> usize {
		self.vals.len()
	}
	pub fn is_empty(&self) -> bool {
		self.vals.is_empty()
	}
	#[inline]
	pub fn contains_val<Q: Borrow<G>>(&self, g: &Q) -> bool {
		self.keys.contains_key(g)
	}
	#[inline]
	pub fn contains_idx(&self, i: usize) -> bool {
		self.vals.contains_key(&i)
	}
	#[inline]
	pub fn idx<Q: Borrow<G>>(&self, g: &Q) -> Option<usize> {
		self.keys.get(g).copied()
	}
	#[inline]
	pub fn val(&self, i: usize) -> Option<&G> {
		self.vals.get(&i)
	}
	#[inline]
	pub fn insert(&mut self, g: G) -> usize {
		match self.keys.get(&g) {
			Some(&i) => { i }
			None => {
				if self.next == usize::MAX {
					panic!("CombIndex can hold at most {} items!", usize::MAX - 1);
				}
				let i = self.next;
				self.next += 1;
				self.vals.insert(i, g.clone());
				self.keys.insert_unchecked(g, i);
				i
			}
		}
	}
	#[inline]
	pub fn remove_val<Q: Borrow<G>>(&mut self, g: &Q) -> Option<(usize, G)> {
		if let Some(i) = self.keys.remove(g) {
			let g = self.vals.remove(&i).unwrap();
			Strategy::on_remove(self, i);
			Some((i, g))
		} else {
			None
		}
	}
	#[inline]
	pub fn remove_idx(&mut self, i: usize) -> Option<(usize, G)> {
		if let Some(g) = self.vals.remove(&i) {
			let i = self.keys.remove(&g).unwrap();
			Strategy::on_remove(self, i);
			Some((i, g))
		} else {
			None
		}
	}
	pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
		self.into_iter()
	}
	pub fn retain<F: Fn(&G) -> bool>(&mut self, f: F) {
		let removed: Vec<usize> = self.iter()
			.filter(|(_, g)| f(g))
			.map(|(i, _)| i)
			.collect();
		removed.into_iter().for_each(|i| { self.remove_idx(i); });
	}
}
#[allow(private_bounds)]
#[cfg(any(feature = "rayon", doc))]
impl<G: CombEq + Clone + Send + Sync, Strategy: IndexStrategy> CombIndex<G, Strategy> {
	#[inline]
	pub fn par_contains_val(&self, g: &G) -> bool {
		self.keys.par_contains_key(g)
	}
	#[inline]
	pub fn par_idx<Q: Borrow<G>>(&self, g: &Q) -> Option<usize> {
		self.keys.par_get(g).copied()
	}
	#[inline]
	pub fn par_insert(&mut self, g: G) -> usize {
		match self.keys.par_get(&g) {
			Some(&i) => { i }
			None => {
				if self.next == usize::MAX {
					panic!("CombIndex can hold at most {} items!", usize::MAX - 1);
				}
				let i = self.next;
				self.next += 1;
				self.vals.insert(i, g.clone());
				self.keys.insert_unchecked(g, i);
				i
			}
		}
	}
	#[inline]
	pub fn par_remove_val<Q: Borrow<G>>(&mut self, g: &Q) -> Option<(usize, G)> {
		if let Some(i) = self.keys.par_remove(g) {
			let g = self.vals.remove(&i).unwrap();
			Strategy::on_par_remove(self, i);
			Some((i, g))
		} else {
			None
		}
	}
	#[inline]
	pub fn par_remove_idx(&mut self, i: usize) -> Option<(usize, G)> {
		if let Some(g) = self.vals.remove(&i) {
			let i = self.keys.par_remove(&g).unwrap();
			Strategy::on_par_remove(self, i);
			Some((i, g))
		} else {
			None
		}
	}
	pub fn par_retain<F: Fn(&G) -> bool + Sync>(&mut self, f: F) {
		let removed: Vec<usize> = self.par_iter()
			.filter(|(_, g)| f(g))
			.map(|(i, _)| i)
			.collect();
		removed.into_iter().for_each(|i| { self.par_remove_idx(i); });
	}
}

// HashIndex

#[allow(private_bounds)]
#[derive(Clone)]
pub struct HashIndex<G: Hash + Eq, Strategy: IndexStrategy = KeepIndex> {
	keys: HashMap<G, usize>,
	vals: HashMap<usize, G>,
	next: usize,
	strategy: PhantomData<Strategy>
}
impl<G: Hash + Eq, Strategy: IndexStrategy> IndexExt<G> for HashIndex<G, Strategy> {
	fn next_index(&self) -> usize {
	    self.next
	}
	fn set_next_index(&mut self, idx: usize) {
	    self.next = idx;
	}
	fn reassign(&mut self, src: usize, dst: usize) {
		let h = self.vals.remove(&src).unwrap();
		*self.keys.get_mut(&h).unwrap() = dst;
		self.vals.insert(dst, h);
	}
	#[cfg(feature = "rayon")]
	fn par_reassign(&mut self, src: usize, dst: usize) where G: Send + Sync {
		self.reassign(src, dst);
	}
}
impl<G: Hash + Eq, Strategy: IndexStrategy> Default for HashIndex<G, Strategy> {
	fn default() -> Self {
		Self { keys: HashMap::new(), vals: HashMap::new(), next: 0, strategy: Default::default() }
	}
}
impl<G: Hash + Eq + Debug, Strategy: IndexStrategy> Debug for HashIndex<G, Strategy> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.debug_map().entries(self.vals.iter()).finish()
	}
}
impl<G: Hash + Eq + Clone, Strategy: IndexStrategy> FromIterator<G> for HashIndex<G, Strategy> {
	fn from_iter<I: IntoIterator<Item=G>>(it: I) -> Self {
		let mut index = Self::new();
		index.extend(it);
		index
	}
}
impl<G: Hash + Eq + Clone, Strategy: IndexStrategy> Extend<G> for HashIndex<G, Strategy> {
	fn extend<I: IntoIterator<Item=G>>(&mut self, it: I) {
		for g in it {
			self.insert(g);
		}
	}
}
impl<G: Hash + Eq, Strategy: IndexStrategy> IntoIterator for HashIndex<G, Strategy> {
	type IntoIter = std::collections::hash_map::IntoIter<usize, G>;
	type Item = (usize, G);
	fn into_iter(self) -> Self::IntoIter {
		self.vals.into_iter()
	}
}
impl<'a, G: Hash + Eq, Strategy: IndexStrategy> IntoIterator for &'a HashIndex<G, Strategy> {
	type IntoIter = std::iter::Map<std::collections::hash_map::Iter<'a, usize, G>, fn((&'a usize, &'a G)) -> (usize, &'a G)>;
	type Item = (usize, &'a G);
	fn into_iter(self) -> Self::IntoIter {
		self.vals.iter().map(|(&i, g)| (i, g))
	}
}
impl<'a, G: Hash + Eq, Strategy: IndexStrategy> IntoIterator for &'a mut HashIndex<G, Strategy> {
	type IntoIter = std::iter::Map<std::collections::hash_map::Iter<'a, usize, G>, fn((&'a usize, &'a G)) -> (usize, &'a G)>;
	type Item = (usize, &'a G);
	fn into_iter(self) -> Self::IntoIter {
		self.vals.iter().map(|(&i, g)| (i, g))
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: Hash + Eq + Clone + Send + Sync, Strategy: IndexStrategy> FromParallelIterator<G> for HashIndex<G, Strategy> {
	fn from_par_iter<I: IntoParallelIterator<Item=G>>(par_iter: I) -> Self {
		let set: HashSet<G> = par_iter.into_par_iter().collect();
		let mut next = 0;
		let keys: HashMap<G, usize> = set.into_iter()
			.map(|g| {
				let i = next;
				next += 1;
				(g, i)
			})
			.collect();
		let vals: HashMap<usize, G> = keys.iter()
			.map(|(g, &i)| (i, g.clone()))
			.collect();
		Self { keys, vals, next, strategy: Default::default() }
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: Hash + Eq + Clone + Send + Sync, Strategy: IndexStrategy> ParallelExtend<G> for HashIndex<G, Strategy> {
	fn par_extend<I: IntoParallelIterator<Item=G>>(&mut self, par_iter: I) {
		let set: HashSet<G> = par_iter.into_par_iter().collect();
		self.extend(set);
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: Hash + Eq + Send, Strategy: IndexStrategy> IntoParallelIterator for HashIndex<G, Strategy> {
	type Iter = rayon::collections::hash_map::IntoIter<usize, G>;
	type Item = (usize, G);
	fn into_par_iter(self) -> Self::Iter {
		self.vals.into_par_iter()
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<'a, G: Hash + Eq + Sync, Strategy: IndexStrategy> IntoParallelIterator for &'a HashIndex<G, Strategy> {
	type Iter = rayon::iter::Map<rayon::collections::hash_map::Iter<'a, usize, G>, fn((&'a usize, &'a G)) -> (usize, &'a G)>;
	type Item = (usize, &'a G);
	fn into_par_iter(self) -> Self::Iter {
		self.vals.par_iter().map(|(&i, g)| (i, g))
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<'a, G: Hash + Eq + Sync, Strategy: IndexStrategy> IntoParallelIterator for &'a mut HashIndex<G, Strategy> {
	type Iter = rayon::iter::Map<rayon::collections::hash_map::Iter<'a, usize, G>, fn((&'a usize, &'a G)) -> (usize, &'a G)>;
	type Item = (usize, &'a G);
	fn into_par_iter(self) -> Self::Iter {
		self.vals.par_iter().map(|(&i, g)| (i, g))
	}
}
impl<G: Hash + Eq + Clone, Strategy: IndexStrategy> std::ops::Index<usize> for HashIndex<G, Strategy> {
	type Output = G;
	#[inline]
	fn index(&self, key: usize) -> &Self::Output {
		self.val(key).expect("no value found for index")
	}
}
#[allow(private_bounds)]
impl<G: Hash + Eq + Clone, Strategy: IndexStrategy> HashIndex<G, Strategy> {
	pub fn new() -> Self {
		Self::default()
	}
	pub fn len(&self) -> usize {
		self.vals.len()
	}
	pub fn is_empty(&self) -> bool {
		self.vals.is_empty()
	}
	#[inline]
	pub fn contains_val<Q: Borrow<G>>(&self, g: &Q) -> bool {
		self.keys.contains_key(g.borrow())
	}
	#[inline]
	pub fn contains_idx(&self, i: usize) -> bool {
		self.vals.contains_key(&i)
	}
	#[inline]
	pub fn idx<Q: Borrow<G>>(&self, g: &Q) -> Option<usize> {
		self.keys.get(g.borrow()).copied()
	}
	#[inline]
	pub fn val(&self, i: usize) -> Option<&G> {
		self.vals.get(&i)
	}
	#[inline]
	pub fn insert(&mut self, g: G) -> usize {
		match self.keys.get(&g) {
			Some(&i) => { i }
			None => {
				if self.next == usize::MAX {
					panic!("HashIndex can hold at most {} items!", usize::MAX - 1);
				}
				let i = self.next;
				self.next += 1;
				self.vals.insert(i, g.clone());
				self.keys.insert(g, i);
				i
			}
		}
	}
	#[inline]
	pub fn remove_val<Q: Borrow<G>>(&mut self, g: &Q) -> Option<(usize, G)> {
		if let Some(i) = self.keys.remove(g.borrow()) {
			let g = self.vals.remove(&i).unwrap();
			Strategy::on_remove(self, i);
			Some((i, g))
		} else {
			None
		}
	}
	#[inline]
	pub fn remove_idx(&mut self, i: usize) -> Option<(usize, G)> {
		if let Some(g) = self.vals.remove(&i) {
			let i = self.keys.remove(&g).unwrap();
			Strategy::on_remove(self, i);
			Some((i, g))
		} else {
			None
		}
	}
	pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
		self.into_iter()
	}
	pub fn retain<F: Fn(&G) -> bool>(&mut self, f: F) {
		let removed: Vec<usize> = self.iter()
			.filter(|(_, g)| f(g))
			.map(|(i, _)| i)
			.collect();
		removed.into_iter().for_each(|i| { self.remove_idx(i); });
	}
}
