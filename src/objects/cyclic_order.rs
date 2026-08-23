use std::fmt::{Debug, Display};
use std::hash::Hash;

use itertools::Itertools;


#[derive(Clone, Debug, Hash)]
pub struct CyclicOrder<T: Clone + Copy + Debug + Display + PartialEq + Eq + Hash> {
	pub data: Vec<T>
}
impl<T: Clone + Copy + Debug + Display + PartialEq + Eq + Hash> Display for CyclicOrder<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "({})", self.data.iter().map(|x| format!("{}", x)).join(", "))
	}
}
impl<T: Clone + Copy + Debug + Display + PartialEq + Eq + Hash> PartialEq for CyclicOrder<T> {
	fn eq(&self, other: &Self) -> bool {
		let n = self.data.len();
		if n != other.data.len() { return false; }
		if n == 0 { return true; }
		'offset:
		for offset in 0..n {
			for i in 0..n {
				let j = (offset + i) % n;
				if self.data[i] != other.data[j] {
					continue 'offset;
				}
			}
			return true;
		}
		return false;
	}
}
impl<T: Clone + Copy + Debug + Display + PartialEq + Eq + Hash> Eq for CyclicOrder<T> {}
impl<T: Clone + Copy + Debug + Display + PartialEq + Eq + Hash> Default for CyclicOrder<T> {
	fn default() -> Self {
		Self { data: vec![] }
	}
}
impl<T: Clone + Copy + Debug + Display + PartialEq + Eq + Hash> CyclicOrder<T> {
	pub fn insert(&mut self, x: T, after: Option<T>) {
		let i = if let Some(after) = after {
			1 + self.data.iter()
				.position(|&y| after == y)
				.expect(&format!("anchor {} not in cyclic ordering {}", after, self))
		} else {
			self.data.len()
		};
		self.data.insert(i, x);
	}
	pub fn len(&self) -> usize {
		self.data.len()
	}
	pub fn replace(&mut self, x: T, y: T) {
		let i = self.data.iter()
			.position(|&z| x == z)
			.expect(&format!("element {} not in cyclic ordering {}", x, self));
		self.data[i] = y;
	}
	pub fn remove(&mut self, x: T) {
		let i = self.data.iter()
			.position(|&y| x == y)
			.expect(&format!("element {} not in cyclic ordering {}", x, self));
		self.data.remove(i);
	}
	pub fn permutations(&self) -> impl Iterator<Item=Self> + Clone {
		let n = self.len();
		(1..n).permutations(n - 1)
			.map(|perm| Self {
				data: std::iter::once(self.data[0])
					.chain(perm.into_iter().map(|i| self.data[i]))
					.collect()
			})
			.dedup()
	}
	pub fn next(&self, x: T) -> T {
		let i = self.data.iter()
			.position(|&y| x == y)
			.expect(&format!("element {} not in cyclic ordering {}", x, self));
		self.data[(i + 1) % self.data.len()]
	}
	pub fn iter_from(&self, x: T) -> impl Iterator<Item=T> {
		let i = self.data.iter()
			.position(|&y| x == y)
			.expect(&format!("element {} not in cyclic ordering {}", x, self));
		self.data.iter().skip(i).cloned().chain(self.data.iter().take(i).cloned())
	}
	pub fn filter_map<T2: Clone + Copy + Debug + Display + PartialEq + Eq + Hash, F: FnMut(&T) -> Option<T2>>(&self, f: F) -> CyclicOrder<T2> {
		CyclicOrder {
			data: self.data.iter().filter_map(f).collect()
		}
	}

	pub fn rev(&self) -> Self {
		Self { data: self.data.iter().rev().copied().collect() }
	}
}
