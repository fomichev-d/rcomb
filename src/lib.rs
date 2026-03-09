#[cfg(feature = "rayon")]
pub use rayon;

pub mod object;
pub mod collections;

#[cfg(test)]
mod tests {
    use super::object::*;
    use super::collections::*;

    #[test]
    fn it_works() {
        assert!(true);
    }
}
