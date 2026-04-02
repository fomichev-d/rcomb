use crate::CombEnum;

use std::fmt::Display;

pub trait CombCsv: Sized {
	type Err;
	const CSV_HEADER: &'static str;
	fn to_csv_string(&self) -> String;
	fn from_csv_string<S: AsRef<str>>(s: S) -> Result<Self, Self::Err>;
}

pub struct CsvColumn<'a, G, T> {
	header: String,
	fmt: Box<dyn Fn(&G, &T) -> String + Send + Sync + 'a>,
	filter: Option<Box<dyn Fn(&str) -> bool + 'a>>
}
impl<'a, G, T> CsvColumn<'a, G, T> {
	pub fn skip<S: Into<String>>(header: S) -> Self {
		Self {
			header: header.into(),
			fmt: Box::new(|_, _| panic!("CsvColumn::skip() can only be used when reading a table!")),
			filter: None
		}
	}
	pub fn filter<F: Fn(&str) -> bool + 'a>(mut self, filter: F) -> Self {
		self.filter = Some(Box::new(filter));
		self
	}
	pub fn for_map<S: Into<String>, F: Fn(&G, &T) -> String + Send + Sync + 'a>(header: S, fmt: F) -> Self {
		Self { header: header.into(), fmt: Box::new(fmt), filter: None }
	}
}
impl<'a, G> CsvColumn<'a, G, ()> {
	pub fn for_set<S: Into<String>, F: Fn(&G) -> String + Send + Sync + 'a>(header: S, fmt: F) -> Self where G: 'static {
		Self {
			header: header.into(),
			fmt: Box::new(move |g, ()| fmt(g)),
			filter: None
		}
	}
}

pub struct CsvConfig<'a, G: CombCsv, T> {
	pub(crate) use_header: bool,
	pub(crate) use_tqdm: bool,
	pub(crate) filename: String,
	pub(crate) dedup: bool,
	columns_pre: Vec<CsvColumn<'a, G, T>>,
	key_idx: usize,
	key_header: String,
	key_filter: Option<Box<dyn Fn(&G) -> bool + 'a>>,
	columns_mid: Vec<CsvColumn<'a, G, T>>,
	value_idx: Option<usize>,
	value_header: Option<String>,
	value_fmt: Option<Box<dyn Fn(&T) -> String + Send + Sync + 'a>>,
	value_parser: Option<Box<dyn Fn(&str) -> T + 'a>>,
	value_filter: Option<Box<dyn Fn(&T) -> bool + 'a>>,
	columns_post: Vec<CsvColumn<'a, G, T>>
}
// TODO: tqdm config
unsafe impl<'a, G: CombCsv, T> Send for CsvConfig<'a, G, T> {}
unsafe impl<'a, G: CombCsv, T> Sync for CsvConfig<'a, G, T> {}
impl<'a, G: CombCsv, T> CsvConfig<'a, G, T> {
	pub fn new<S: Into<String>>(filename: S) -> Self {
		Self {
			use_header: false,
			use_tqdm: false,
			filename: filename.into(),
			dedup: false,
			columns_pre: vec![],
			key_idx: 0,
			key_header: G::CSV_HEADER.into(),
			key_filter: None,
			columns_mid: vec![],
			value_idx: None,
			value_header: None,
			value_fmt: None,
			value_parser: None,
			value_filter: None,
			columns_post: vec![],
		}
	}
	pub fn use_header(mut self) -> Self {
		self.use_header = true;
		self
	}
	pub fn tqdm(mut self) -> Self {
		self.use_tqdm = true;
		self
	}
	pub fn dedup(mut self) -> Self {
		self.dedup = true;
		self
	}
	pub fn key_header<S: Into<String>>(mut self, key_header: S) -> Self {
		self.key_header = key_header.into();
		self
	}
	pub fn filter_key<F: Fn(&G) -> bool + 'a>(mut self, filter: F) -> Self {
		self.key_filter = Some(Box::new(filter));
		self
	}
	pub fn filter_value<F: Fn(&T) -> bool + 'a>(mut self, filter: F) -> Self {
		self.value_filter = Some(Box::new(filter));
		self
	}
	pub fn fmt_value<S: Into<String>, F: Fn(&T) -> String + Send + Sync + 'a>(mut self, header: S, fmt: F) -> Self {
		self.value_header = Some(header.into());
		self.value_fmt = Some(Box::new(fmt));
		self.value_idx = Some(self.columns_pre.len() + 1 + self.columns_mid.len());
		self
	}
	pub fn display_value<S: Into<String>>(mut self, header: S) -> Self where T: Display {
		self.value_header = Some(header.into());
		self.value_fmt = Some(Box::new(|val| val.to_string()));
		self.value_idx = Some(self.columns_pre.len() + 1 + self.columns_mid.len());
		self
	}
	pub fn parse_value<S: Into<String>, F: Fn(&str) -> T + 'a>(mut self, header: S, parser: F) -> Self {
		self.value_header = Some(header.into());
		self.value_parser = Some(Box::new(parser));
		self.value_idx = Some(self.columns_pre.len() + 1 + self.columns_mid.len());
		self
	}
	pub fn columns<
		I1: IntoIterator<Item=CsvColumn<'a, G, T>>,
		I2: IntoIterator<Item=CsvColumn<'a, G, T>>,
		I3: IntoIterator<Item=CsvColumn<'a, G, T>>
	>(mut self, columns_pre: I1, columns_mid: I2, columns_post: I3) -> Self {
		self.columns_pre = columns_pre.into_iter().collect();
		self.columns_mid = columns_mid.into_iter().collect();
		self.columns_post = columns_post.into_iter().collect();
		self.key_idx = self.columns_pre.len();
		self.value_idx = self.value_idx.map(|_| self.columns_pre.len() + 1 + self.columns_mid.len());
		self
	}

	pub(crate) fn write_header(&self) -> Vec<&str> {
		self.columns_pre.iter().map(|column| column.header.as_str())
			.chain(std::iter::once(self.key_header.as_str()))
			.chain(self.columns_mid.iter().map(|column| column.header.as_str()))
			.chain(self.value_header.iter().map(|header| header.as_str()))
			.chain(self.columns_post.iter().map(|column| column.header.as_str()))
			.collect()
	}
	pub(crate) fn write_entry(&self, g: &G, val: &T) -> Option<Vec<String>> {
		if let Some(ref filter) = self.key_filter {
			if !filter(g) { return None; }
		}
		if let Some(ref filter) = self.value_filter {
			if !filter(val) { return None; }
		}
		let entry: Vec<String> = self.columns_pre.iter().map(|column| (column.fmt)(g, val))
			.chain(std::iter::once(g.to_csv_string()))
			.chain(self.columns_mid.iter().map(|column| (column.fmt)(g, val)))
			.chain(self.value_fmt.iter().map(|fmt| (fmt)(val)))
			.chain(self.columns_post.iter().map(|column| (column.fmt)(g, val)))
			.collect();
		for (i, column) in self.columns_pre.iter().enumerate() {
			if let Some(ref filter) = column.filter {
				if !filter(&entry[i]) { return None; }
			}
		}
		for (i, column) in self.columns_mid.iter().enumerate() {
			if let Some(ref filter) = column.filter {
				if !filter(&entry[self.key_idx + 1 + i]) { return None; }
			}
		}
		let value_idx = self.value_idx.unwrap_or(self.key_idx + 1 + self.columns_mid.len());
		for (i, column) in self.columns_post.iter().enumerate() {
			if let Some(ref filter) = column.filter {
				if !filter(&entry[value_idx + 1 + i]) { return None; }
			}
		}
		Some(entry)
	}
	pub(crate) fn read_entry(&self, entry: &csv::StringRecord) -> Option<(G, Option<T>)> {
		for (i, column) in self.columns_pre.iter().enumerate() {
			if let Some(ref filter) = column.filter {
				if !filter(&entry[i]) { return None; }
			}
		}
		for (i, column) in self.columns_mid.iter().enumerate() {
			if let Some(ref filter) = column.filter {
				if !filter(&entry[self.key_idx + 1 + i]) { return None; }
			}
		}
		let value_idx = self.value_idx.unwrap_or(self.key_idx + 1 + self.columns_mid.len());
		for (i, column) in self.columns_post.iter().enumerate() {
			if let Some(ref filter) = column.filter {
				if !filter(&entry[value_idx + 1 + i]) { return None; }
			}
		}
		let g = G::from_csv_string(&entry[self.key_idx]).ok()?;
		if let Some(ref filter) = self.key_filter {
			if !filter(&g) { return None; }
		}
		let val = if let Some(idx) = self.value_idx && let Some(ref parser) = self.value_parser {
			let val = parser(&entry[idx]);
			if let Some(ref filter) = self.value_filter {
				if !filter(&val) { return None; }
			}
			Some(val)
		} else {
			None
		};
		Some((g, val))
	}
}

pub trait CollectionCsvExt<G, T>: Sized {
	fn read_csv(config: CsvConfig<G, T>) -> std::io::Result<Self> where G: CombCsv;
	fn save_csv(&self, config: CsvConfig<G, T>) -> std::io::Result<()> where G: CombCsv;

	#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
	#[cfg(feature = "rayon")]
	fn par_read_csv(config: CsvConfig<G, T>) -> std::io::Result<Self> where G: CombCsv + Send + Sync, T: Send + Sync;
	#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
	#[cfg(feature = "rayon")]
	fn save_ord_csv(&self, config: CsvConfig<G, T>) -> std::io::Result<()> where G: CombCsv + CombEnum<usize> + Send + Sync, G::Iter: Send + Sync, T: Send + Sync;
	#[cfg(not(feature = "rayon"))]
	fn save_ord_csv(&self, config: CsvConfig<G, T>) -> std::io::Result<()> where G: CombCsv + CombEnum<usize>;
}
