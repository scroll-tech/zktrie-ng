//! KeyHasher is a trait that helps to utilize cache while hashing keys.
use crate::hash::{HashScheme, ZkHash};
use std::error::Error;

mod no_cache;
pub use no_cache::*;

mod ref_cache;
pub use ref_cache::*;

mod sync_cache;
pub use sync_cache::*;

/// Error type for KeyCacheDb
#[derive(Debug, thiserror::Error)]
pub enum KeyHasherError<HashErr> {
    /// Error when hashing
    #[error(transparent)]
    Hash(HashErr),
    /// Other Error
    #[error(transparent)]
    Other(Box<dyn Error>),
}

/// Hasher for keys.
pub trait KeyHasher<H: HashScheme> {
    /// Hash a key.
    fn hash(&self, key: &[u8]) -> Result<ZkHash, KeyHasherError<H::Error>> {
        H::hash_bytes(key).map_err(KeyHasherError::Hash)
    }
}
