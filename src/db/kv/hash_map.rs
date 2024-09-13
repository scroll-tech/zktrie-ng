//! KVDatabase in-memory implementation using a [`HashMap`](std::collections::HashMap).
use super::KVDatabase;
use crate::HashMap;
use std::convert::Infallible;
use std::fmt::Debug;

/// A simple in-memory key-value store backed by a [`HashMap`](std::collections::HashMap).
///
/// It's intended to be not [`Clone`], since [`Clone::clone`] will clone the entire [`HashMapDb`].
///
/// If you need to clone the entire database,
/// you can use [`HashMapDb::inner`] to get the inner [`HashMapDb`],
/// and then clone the [`HashMapDb`] manually and create a new via [`HashMapDb::from_map`].
#[derive(Default)]
pub struct HashMapDb {
    db: HashMap<Box<[u8]>, Box<[u8]>>,
}

impl HashMapDb {
    /// Create a new empty `HashMapDb`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new `BTreeMapDb` from a `BTreeMap`.
    pub fn from_map(db: HashMap<Box<[u8]>, Box<[u8]>>) -> Self {
        Self { db }
    }

    /// Get the inner `BTreeMap`.
    pub fn inner(&self) -> &HashMap<Box<[u8]>, Box<[u8]>> {
        &self.db
    }

    /// Get the inner `BTreeMap`.
    pub fn into_inner(self) -> HashMap<Box<[u8]>, Box<[u8]>> {
        self.db
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
