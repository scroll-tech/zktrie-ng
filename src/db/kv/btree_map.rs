//! KVDatabase in-memory implementation using a [`BTreeMap`].
use super::KVDatabase;
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt::Debug;

/// A simple in-memory key-value store backed by a `BTreeMap`.
///
/// It's intended to be not `Clone`, since [`Clone::clone`] will clone the entire [`BTreeMap`].
///
/// If you need to clone the entire database,
/// you can use [`BTreeMapDb::inner`] to get the inner [`BTreeMap`],
/// and then clone the [`BTreeMap`] manually and create a new via [`BTreeMapDb::from_map`].
#[derive(Default)]
pub struct BTreeMapDb {
    db: BTreeMap<Box<[u8]>, Box<[u8]>>,
}

impl BTreeMapDb {
    /// Create a new empty `BTreeMapDb`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new `BTreeMapDb` from a `BTreeMap`.
    pub fn from_map(db: BTreeMap<Box<[u8]>, Box<[u8]>>) -> Self {
        Self { db }
    }

    /// Get the inner `BTreeMap`.
    pub fn inner(&self) -> &BTreeMap<Box<[u8]>, Box<[u8]>> {
        &self.db
    }

    /// Get the inner `BTreeMap`.
    pub fn into_inner(self) -> BTreeMap<Box<[u8]>, Box<[u8]>> {
        self.db
    }
}

impl Debug for BTreeMapDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BTreeMapDb").field(&self.db.len()).finish()
    }
}

impl KVDatabase for BTreeMapDb {
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
