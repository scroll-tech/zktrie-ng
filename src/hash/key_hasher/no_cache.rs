use crate::hash::{key_hasher::KeyHasher, HashScheme};

#[derive(Copy, Clone, Debug, Default)]
pub struct NoCacheHasher;

impl<H: HashScheme> KeyHasher<H> for NoCacheHasher {}
