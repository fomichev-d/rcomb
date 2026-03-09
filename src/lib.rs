#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
#[cfg(feature = "rayon")]
pub use rayon;

#[cfg_attr(docsrs, doc(cfg(feature = "petgraph")))]
#[cfg(feature = "petgraph")]
pub use petgraph;

pub mod objects;
pub mod collections;

#[cfg(test)]
mod tests {
    use super::objects::*;
    use super::collections::*;

    #[test]
    fn it_works() {
        assert!(true);
    }
}
