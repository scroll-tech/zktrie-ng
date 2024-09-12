//! KVDatabase in-memory implementation using a [`BTreeMap`].
use super::KVDatabase;
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt::Debug;

/// A simple in-memory key-value store backed by a `HashMap`.
#[derive(Clone, Default)]
pub struct HashMapDb {
    db: crate::HashMap<Box<[u8]>, Box<[u8]>>,
}

impl HashMapDb {
    /// Create a new empty `HashMapDb`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Debug for HashMapDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("HashMapDb").field(&self.db.len()).finish()
    }
}

impl KVDatabase for HashMapDb {
    type Error = Infallible;

    fn put_owned(
        &mut self,
        k: Box<[u8]>,
        v: Box<[u8]>,
    ) -> Result<Option<impl AsRef<[u8]>>, Self::Error> {
        Ok(self.db.insert(k, v))
    }

    fn get(&self, k: &[u8]) -> Result<Option<impl AsRef<[u8]>>, Self::Error> {
        Ok(self.db.get(k))
    }

    fn remove(&mut self, k: &[u8]) -> Result<(), Self::Error> {
        self.db.remove(k);
        Ok(())
    }

    fn extend<T: IntoIterator<Item = (Box<[u8]>, Box<[u8]>)>>(
        &mut self,
        other: T,
    ) -> Result<(), Self::Error> {
        self.db.extend(other);
        Ok(())
    }
}
