pub fn binomial(n: u128, mut k: u128) -> u128 {
	if k > n { return 0; }
	k = u128::min(k, n - k);
	let mut c_nk = 1;
	for i in 1..=k {
		c_nk *= n + 1 - i;
		c_nk /= i;
	}
	c_nk
}

pub fn double_factorial(n: u128) -> u128 {
	// the case of overflow, this is actually -1
	if n == u128::MAX { return 1; }
	let k0 = if n % 2 == 0 { 2 } else { 1 };
	(k0..=n).step_by(2).product()
}

pub fn euler_phi(mut n: u128) -> u128 {
	let factorisation = primefactor::PrimeFactors::factorize(n);
	let primes = factorisation.factors().iter().map(|factor| factor.integer);
	for p in primes {
		n -= n / p;
	}
	n
}

pub fn divisors(n: u128) -> Vec<u128> {
	use divisors::get_divisors;

	let mut divisors = vec![1];
	divisors.extend(get_divisors(n));
	if divisors.iter().last() != Some(&n) {
		divisors.push(n);
	}
	divisors
}

#[cfg(feature = "rayon")]
#[inline]
pub fn entry_value<G, T>(tuple: (G, T)) -> T { tuple.1 }

#[cfg(feature = "rayon")]
pub fn par_dedup<T: Sync, F: Fn(&T, &T) -> bool + Sync>(data: &mut Vec<T>, eq: F) {
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

pub trait Sealed {}

#[derive(Clone, Debug)]
pub struct SizedIter<I> {
	pub iter: I,
	pub n_remaining: Option<usize>
}
impl<I: Iterator> Iterator for SizedIter<I> {
	type Item = I::Item;
	fn next(&mut self) -> Option<Self::Item> {
		let item = self.iter.next();
		self.n_remaining = self.n_remaining.map(|n| n.saturating_sub(1));
		item
	}
	fn size_hint(&self) -> (usize, Option<usize>) {
		if let Some(n) = self.n_remaining {
			(n, Some(n))
		} else {
			(0, None)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_binomial() {
		let n = 10;
		let vals: Vec<u128> = (0..=n).map(|k| binomial(n, k)).collect();
		assert_eq!(vals, vec![1, 10, 45, 120, 210, 252, 210, 120, 45, 10, 1]);
	}
	#[test]
	fn test_double_factorial() {
		assert_eq!(double_factorial(0u128.overflowing_sub(1).0), 1);
		assert_eq!(double_factorial(0), 1);
		assert_eq!(double_factorial(1), 1);
		assert_eq!(double_factorial(16), 10321920);
		assert_eq!(double_factorial(17), 34459425);
	}
	#[test]
	fn test_euler_phi() {
		assert_eq!(euler_phi(1), 1);
		assert_eq!(euler_phi(2), 1);
		assert_eq!(euler_phi(12843), 8556);
		assert_eq!(euler_phi(1010102), 505050);
	}
	#[test]
	fn test_divisors() {
		assert_eq!(divisors(2u128), vec![1, 2]);
		assert_eq!(divisors(141u128), vec![1, 3, 47, 141]);
		assert_eq!(divisors(143u128), vec![1, 11, 13, 143]);
	}
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
