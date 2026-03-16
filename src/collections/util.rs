#[inline]
pub(crate) fn entry_key<G, T>(tuple: (G, T)) -> G { tuple.0 }
#[inline]
pub(crate) fn entry_value<G, T>(tuple: (G, T)) -> T { tuple.1 }
#[inline]
pub(crate) fn move_refs<'a, G, T>(tuple: &'a (G, T)) -> (&'a G, &'a T) { (&tuple.0, &tuple.1) }
#[inline]
pub(crate) fn move_refs_mut2<'a, G, T>(tuple: &'a mut (G, T)) -> (&'a G, &'a mut T) { (&tuple.0, &mut tuple.1) }

#[cfg(feature = "rayon")]
pub(crate) fn par_dedup<T: Sync, F: Fn(&T, &T) -> bool + Sync>(data: &mut Vec<T>, eq: F) {
	use rayon::iter::{ParallelIterator, ParallelBridge};
	use uf_rush::UFRush;

	let n = data.len();
	let uf = UFRush::new(n);

	(0..n).flat_map(|i| (i + 1..n).map(move |j| (i, j)))
		.par_bridge()
		.for_each(|(i, j)| {
			if uf.same(i, j) { return; }
			if eq(&data[i], &data[j]) {
				uf.unite(i, j);
			}
		});
	
	*data = data.drain(..)
		.enumerate()
		.filter(|&(i, _)| uf.find(i) == i)
		.map(|(_, x)| x)
		.collect();
}

#[cfg(test)]
mod tests {
	use super::*;
	#[cfg(feature = "rayon")]
	#[test]
	fn par_dedup_modulo() {
		let n = 5000;
		let modulo = 149;
		let mut data: Vec<_> = (0..n).collect();
		par_dedup(&mut data, |&i, &j| { i % modulo == j % modulo });
		data = data.into_iter().map(|i| i % modulo).collect();
		data.sort();
		assert_eq!(data, (0..modulo).collect::<Vec<_>>());
	}
	#[cfg(feature = "rayon")]
	#[test]
	fn par_dedup_unique() {
		let n = 5000;
		let mut data: Vec<_> = (0..n).collect();
		par_dedup(&mut data, |&i, &j| i == j);
		assert_eq!(data.len(), n);
	}
	#[cfg(feature = "rayon")]
	#[test]
	fn par_dedup_same() {
		let n = 5000;
		let mut data: Vec<()> = vec![(); n];
		par_dedup(&mut data, |&(), &()| true);
		assert_eq!(data.len(), 1);
	}
}
