use std::{collections::HashSet, hash::Hash};

use crate::*;
use crate::collections::map::*;
use crate::io::*;

#[cfg(feature = "rayon")]
use rayon::iter::{
	ParallelBridge,
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
pub struct CombSet<G: CombEq>(CombMap<G, ()>);

impl<G: CombEq> Default for CombSet<G> {
	fn default() -> Self { Self(Default::default()) }
}
impl<G: CombEq + Clone> Clone for CombSet<G> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}
impl<G: CombEq> FromIterator<G> for CombSet<G> {
	fn from_iter<I: IntoIterator<Item=G>>(it: I) -> Self {
		let mut set = Self::new();
		set.extend_unchecked(it);
		set
	}
}
impl<G: CombEq> IntoIterator for CombSet<G> {
	type IntoIter = std::iter::Map<<CombMap::<G, ()> as IntoIterator>::IntoIter, fn((G, ())) -> G>;
	type Item = G;
	fn into_iter(self) -> Self::IntoIter {
		self.0.into_iter().map(|(g, ())| g)
	}
}
impl<'a, G: CombEq> IntoIterator for &'a CombSet<G> {
	type IntoIter = std::iter::Map<<&'a CombMap<G, ()> as IntoIterator>::IntoIter, fn((&'a G, &'a ())) -> &'a G>;
	type Item = &'a G;
	fn into_iter(self) -> Self::IntoIter {
		(&self.0).into_iter().map(|(g, &())| g)
	}
}
impl<'a, G: CombEq> IntoIterator for &'a mut CombSet<G> {
	type IntoIter = std::iter::Map<<&'a mut CombMap<G, ()> as IntoIterator>::IntoIter, fn((&'a G, &'a mut ())) -> &'a G>;
	type Item = &'a G;
	fn into_iter(self) -> Self::IntoIter {
		(&mut self.0).into_iter().map(|(g, &mut ())| g)
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync> FromParallelIterator<G> for CombSet<G> {
	fn from_par_iter<I: IntoParallelIterator<Item=G>>(par_iter: I) -> Self {
		Self(CombMap::<G, ()>::from_par_iter(par_iter.into_par_iter().map(|g| (g, ()))))
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync, H: CombEq<G> + Into<G> + Send> ParallelExtend<H> for CombSet<G> {
	fn par_extend<I: IntoParallelIterator<Item=H>>(&mut self, par_iter: I) {
		self.0.par_extend(par_iter.into_par_iter().map(|g| (g, ())))
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<G: CombEq + Send> IntoParallelIterator for CombSet<G> {
	type Iter = rayon::iter::Map<<CombMap::<G, ()> as IntoParallelIterator>::Iter, fn((G, ())) -> G>;
	type Item = G;
	fn into_par_iter(self) -> Self::Iter {
		self.0.into_par_iter().map(|(g, ())| g)
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<'a, G: CombEq + Sync> IntoParallelIterator for &'a CombSet<G> {
	type Iter = rayon::iter::Map<<&'a CombMap<G, ()> as IntoParallelIterator>::Iter, fn((&'a G, &'a ())) -> &'a G>;
	type Item = &'a G;
	fn into_par_iter(self) -> Self::Iter {
		(&self.0).into_par_iter().map(|(g, &())| g)
	}
}
#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
impl<'a, G: CombEq + Send + Sync> IntoParallelIterator for &'a mut CombSet<G> {
	type Iter = rayon::iter::Map<<&'a mut CombMap<G, ()> as IntoParallelIterator>::Iter, fn((&'a G, &'a mut ())) -> &'a G>;
	type Item = &'a G;
	fn into_par_iter(self) -> Self::Iter {
		(&mut self.0).into_par_iter().map(|(g, &mut ())| g)
	}
}
impl<G: CombEq, H: CombEq<G> + Into<G>> Extend<H> for CombSet<G> {
	#[inline]
	fn extend<I>(&mut self, it: I) where I: IntoIterator<Item=H> {
		self.0.extend(it.into_iter().map(|g| (g, ())))
	}
}
impl<G: CombEq> CollectionCsvExt<G, ()> for CombSet<G> {
	#[inline]
	fn read_csv(config: CsvConfig<G, ()>) -> std::io::Result<Self> where G: CombCsv {
		Ok(Self(CombMap::read_csv(config)?))
	}
	#[inline]
	fn save_csv(&self, config: CsvConfig<G, ()>) -> std::io::Result<()> where G: CombCsv {
		self.0.save_csv(config)
	}

	#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
	#[cfg(feature = "rayon")]
	#[inline]
	fn par_read_csv(config: CsvConfig<G, ()>) -> std::io::Result<Self> where G: CombCsv + Send + Sync {
		Ok(Self(CombMap::par_read_csv(config)?))
	}
	#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
	#[cfg(feature = "rayon")]
	#[inline]
	fn save_ord_csv(&self, config: CsvConfig<G, ()>) -> std::io::Result<()> where G: CombCsv + CombEnum<usize> + Send + Sync, G::Iter: Send + Sync {
		self.0.save_ord_csv(config)
	}
	#[cfg(not(feature = "rayon"))]
	fn save_ord_csv(&self, config: CsvConfig<G, ()>) -> std::io::Result<()> where G: CombCsv + CombEnum<usize> {
		self.0.save_ord_csv(config)
	}
}
impl<G: CombEq> CombSet<G> {
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
	#[inline]
	pub fn is_empty(&self) -> bool { self.0.is_empty() }
	/// Inserts an item into the set.
	///
	/// If the set did have an isomorphic item present, it will not be replaced; this matters for items that can be isomorphic without being identical.
	///
	/// If there are several isomorphic items (e.g. after [`insert_unchecked`](Self::insert_unchecked)), an arbitrary one is picked to be replaced.
	/// To restore key uniqueness, use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup).
	#[inline]
	pub fn insert<H: CombEq<G> + Into<G>>(&mut self, g: H) { self.0.insert(g, ()); }
	/// Inserts an item into the set, assuming it is not isomorphic to any present ones.
	///
	/// If the set did have an isomorphic item present, it will now store several isomorphic items.
	/// To restore key uniqueness, use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup).
	#[inline]
	pub fn insert_unchecked<H: CombEq<G> + Into<G>>(&mut self, g: H) { self.0.insert_unchecked(g, ()) }
	/// Removes an item from the set.
	///
	/// If there are several isomorphic items (e.g. after [`insert_unchecked`](Self::insert_unchecked)), an arbitrary one is picked.
	/// To restore key uniqueness, use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup).
	#[inline]
	pub fn remove<H: CombEq<G>>(&mut self, g: &H) { self.0.remove(g); }
	/// Extends the set with the contents of the iterator, assuming the items are not isomorphic to each other or any present ones.
	///
	/// If the set or the iterator did have isomorphic items, it will now store several isomorphic items.
	/// To restore key uniqueness, use [`dedup`](Self::dedup) or [`par_dedup`](Self::par_dedup).
	#[inline]
	pub fn extend_unchecked<H: CombEq<G> + Into<G>, I: IntoIterator<Item=H>>(&mut self, it: I) { self.0.extend_unchecked(it.into_iter().map(|g| (g, ()))) }
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
	pub fn contains<H: CombEq<G>>(&self, g: &H) -> bool { self.0.contains_key(g) }
	/// An iterator visiting all items in arbitrary order.
	/// The iterator element type is `&'a G`.
	#[inline]
	pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
		self.into_iter()
	}
}
#[cfg(feature = "rayon")]
impl<G: CombEq + Send + Sync> CombSet<G> {
	#[inline]
	pub fn par_insert<H: CombEq<G> + Into<G> + Sync>(&mut self, g: H) {
		self.0.par_insert(g, ());
	}
	#[inline]
	pub fn par_remove<H: CombEq<G> + Sync>(&mut self, g: &H) {
		self.0.par_remove(g);
	}
	#[inline]
	pub fn par_extend_unchecked<H: CombEq<G> + Into<G> + Send, I: IntoParallelIterator<Item=H>>(&mut self, par_iter: I) {
		self.0.par_extend_unchecked(par_iter.into_par_iter().map(|g| (g, ())));
	}
	#[inline]
	pub fn par_retain<F: Fn(&G) -> bool + Copy + Sync>(&mut self, f: F) { self.0.par_retain(|g, ()| f(g)) }
	#[inline]
	pub fn par_contains<H: CombEq<G> + Sync>(&self, g: &H) -> bool { self.0.par_contains_key(g) }
	#[inline]
	pub fn par_dedup(&mut self) { self.0.par_dedup() }
}

impl<G: Eq + Hash> CollectionCsvExt<G, ()> for HashSet<G> {
	fn read_csv(config: CsvConfig<G, ()>) -> std::io::Result<Self> where G: CombCsv {
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

		for (g, _) in it.filter_map(|record| config.read_entry(&record)) {
			map.insert(g);
		}
		Ok(map)
	}
	fn save_csv(&self, config: CsvConfig<G, ()>) -> std::io::Result<()> where G: CombCsv {
		#[cfg(feature = "kdam")]
		use kdam::TqdmIterator;

		let mut writer = csv::Writer::from_path(&config.filename)?;
		if config.use_header { writer.write_record(config.write_header())?; }

		#[allow(unused_mut)]
		let mut it: Box<dyn Iterator<Item=&G>> = Box::new(self.iter());

		#[cfg(feature = "kdam")]
		if config.use_tqdm {
			it = Box::new(it.tqdm());
		}

		for g in it {
			if let Some(entry) = config.write_entry(g, &()) {
				writer.write_record(entry)?;
			}
		}
		Ok(())
	}

	#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
	#[cfg(feature = "rayon")]
	fn par_read_csv(config: CsvConfig<G, ()>) -> std::io::Result<Self> where G: CombCsv + Send + Sync {
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

		let entries: Vec<_> = it.par_bridge()
			.filter_map(|record| config.read_entry(&record))
			.map(|(g, _)| g)
			.collect();
		let mut map = Self::new();
		map.par_extend(entries);
		Ok(map)
	}
	#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
	#[cfg(feature = "rayon")]
	fn save_ord_csv(&self, config: CsvConfig<G, ()>) -> std::io::Result<()> where G: CombCsv + CombEnum<usize> + Send + Sync, G::Iter: Send + Sync {
		#[cfg(feature = "kdam")]
		use kdam::TqdmIterator;
		use rayon::prelude::ParallelSliceMut;

		let max_deg = self.iter()
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
				.filter_map(|(i, g)| config.write_entry(&g, &()).map(|entry| (i, entry)))
				.collect::<Vec<_>>();
			entries.par_sort_unstable_by_key(|&(i, _)| i);
			for (_, entry) in entries {
				writer.write_record(entry)?;
			}
		}
		Ok(())
	}
	#[cfg(not(feature = "rayon"))]
	fn save_ord_csv(&self, config: CsvConfig<G, ()>) -> std::io::Result<()> where G: CombCsv + CombEnum<usize> {
		#[cfg(feature = "kdam")]
		use kdam::TqdmIterator;

		let max_deg = self.iter()
			.map(|g| g.degree())
			.max()
			.unwrap_or(0);

		let mut writer = csv::Writer::from_path(&config.filename)?;
		if config.use_header { writer.write_record(config.write_header())?; }

		for deg in 0..=max_deg {
			#[allow(unused_mut)]
			let mut it: Box<dyn Iterator<Item=(usize, G)>> = Box::new(G::iterate_deg(deg).enumerate());
			#[cfg(feature = "kdam")]
			if config.use_tqdm { it = Box::new(it.tqdm()); }
			let mut entries = it
				.filter_map(|(i, g)| config.write_entry(&g, &()).map(|entry| (i, entry)))
				.collect::<Vec<_>>();
			entries.sort_unstable_by_key(|&(i, _)| i);
			for (_, entry) in entries {
				writer.write_record(entry)?;
			}
		}
		Ok(())
	}
}
