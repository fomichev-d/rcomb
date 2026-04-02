use crate::objects::graph::*;

use std::fmt::Display;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Framing {
	Zero,
	One
}
impl Default for Framing {
	fn default() -> Self {
		Self::Zero
	}
}
impl NodeMatch for Framing {}
impl std::ops::Add for Framing {
	type Output = Self;
	fn add(self, other: Self) -> Self::Output {
		match (self, other) {
			(Self::Zero, Self::Zero) => { Self::Zero }
			(Self::Zero, Self::One) => { Self::One }
			(Self::One, Self::Zero) => { Self::One }
			(Self::One, Self::One) => { Self::Zero }
		}
	}
}
impl std::ops::AddAssign for Framing {
	fn add_assign(&mut self, other: Self) {
		*self = *self + other;
	}
}
impl std::iter::Sum for Framing {
	fn sum<I>(iter: I) -> Self where I: Iterator<Item = Self> {
		let mut v = Framing::Zero;
		for x in iter {
			v += x;
		}
		v
	}
}
impl From<bool> for Framing {
	fn from(v: bool) -> Self {
		if v { Self::One } else { Self::Zero }
	}
}
impl Display for Framing {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Zero => { write!(f, "0") }
			Self::One => { write!(f, "1") }
		}
	}
}

pub type FGraph = Graph<Framing>;

impl Display for FGraph {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{{[")?;
		for (i, v) in self.vertices().enumerate() {
			if i > 0 {
				write!(f, ", ")?;
			}
			write!(f, "{}", self.vertex(v).unwrap())?;
		}
		write!(f, "] [")?;
		for (i, e) in self.edges().enumerate() {
			if i > 0 {
				write!(f, ", ")?;
			}
			write!(f, "({}, {})", e.0.index(), e.1.index())?;
		}
		write!(f, "]}}")
	}
}
