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
    gc_enabled: bool,
    db: HashMap<Box<[u8]>, Box<[u8]>>,
}

impl HashMapDb {
    /// Create a new empty [`HashMapDb`].
    pub fn new(gc_enabled: bool) -> Self {
        Self {
            gc_enabled,
            db: HashMap::new(),
        }
    }

    /// Create a new [`HashMapDb`] from a [`HashMap`](std::collections::HashMap).
    pub fn from_map(gc_enabled: bool, db: HashMap<Box<[u8]>, Box<[u8]>>) -> Self {
        Self { gc_enabled, db }
    }

    /// Get the inner [`HashMap`](std::collections::HashMap).
    pub fn inner(&self) -> &HashMap<Box<[u8]>, Box<[u8]>> {
        &self.db
    }

    /// Into the inner [`HashMap`](std::collections::HashMap).
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

    fn set_gc_enabled(&mut self, gc_enabled: bool) {
        self.gc_enabled = gc_enabled;
    }

    fn gc_enabled(&self) -> bool {
        self.gc_enabled
    }

    fn remove(&mut self, k: &[u8]) -> Result<(), Self::Error> {
        if self.gc_enabled {
            self.db.remove(k);
        } else {
            warn!("garbage collection is disabled, remove is ignored");
        }
        Ok(())
    }

    fn retain<F>(&mut self, mut f: F) -> Result<(), Self::Error>
    where
        F: FnMut(&[u8], &[u8]) -> bool,
    {
        let mut removed = 0;
        self.db.retain(|k, v| {
            let keep = f(k, v);
            if !keep {
                removed += 1;
            }
            keep
        });
        trace!("{} key-value pairs removed", removed);
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
