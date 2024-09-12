use crate::hash::{key_hasher::KeyHasher, HashScheme};

/// A hasher that does not cache the hash.
#[derive(Copy, Clone, Debug, Default)]
pub struct NoCacheHasher;

impl<H: HashScheme> KeyHasher<H> for NoCacheHasher {}
