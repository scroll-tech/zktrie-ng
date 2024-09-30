//! KVDatabase in-memory implementation using a [`HashMap`](std::collections::HashMap).
use super::KVDatabase;
use crate::HashMap;
use alloy_primitives::bytes::Bytes;
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
    db: HashMap<Box<[u8]>, Bytes>,
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
    pub fn from_map(gc_enabled: bool, db: HashMap<Box<[u8]>, Bytes>) -> Self {
        Self { gc_enabled, db }
    }

    /// Get the inner [`HashMap`](std::collections::HashMap).
    pub fn inner(&self) -> &HashMap<Box<[u8]>, Bytes> {
        &self.db
    }

    /// Into the inner [`HashMap`](std::collections::HashMap).
    pub fn into_inner(self) -> HashMap<Box<[u8]>, Bytes> {
        self.db
    }
}

impl Debug for HashMapDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("HashMapDb").field(&self.db.len()).finish()
    }
}

impl KVDatabase for HashMapDb {
    type Item = Bytes;
    type Error = Infallible;

    #[inline]
    fn contains_key(&self, k: &[u8]) -> Result<bool, Self::Error> {
        Ok(self.db.contains_key(k))
    }

    #[inline]
    fn put(&mut self, k: &[u8], v: &[u8]) -> Result<Option<Self::Item>, Self::Error> {
        Ok(self.db.insert(k.into(), Bytes::copy_from_slice(v)))
    }

    #[inline]
    fn or_put(&mut self, k: &[u8], v: &[u8]) -> Result<(), Self::Error> {
        self.db
            .entry(k.into())
            .or_insert_with(|| Bytes::copy_from_slice(v));
        Ok(())
    }

    #[inline]
    fn or_put_with<O: Into<Self::Item>, F: FnOnce() -> O>(
        &mut self,
        k: &[u8],
        default: F,
    ) -> Result<(), Self::Error> {
        self.db.entry(k.into()).or_insert_with(|| default().into());
        Ok(())
    }

    #[inline]
    fn put_owned<K: AsRef<[u8]> + Into<Box<[u8]>>>(
        &mut self,
        k: K,
        v: impl Into<Self::Item>,
    ) -> Result<Option<Self::Item>, Self::Error> {
        Ok(self.db.insert(k.into(), v.into()))
    }

    #[inline]
    fn get<K: AsRef<[u8]> + Clone>(&self, k: K) -> Result<Option<Self::Item>, Self::Error> {
        Ok(self.db.get(k.as_ref()).cloned())
    }

    #[inline]
    fn is_gc_supported(&self) -> bool {
        true
    }

    #[inline]
    fn set_gc_enabled(&mut self, gc_enabled: bool) {
        self.gc_enabled = gc_enabled;
    }

    #[inline]
    fn gc_enabled(&self) -> bool {
        self.gc_enabled
    }

    #[inline]
    fn remove(&mut self, k: &[u8]) -> Result<(), Self::Error> {
        if self.gc_enabled {
            self.db.remove(k);
        } else {
            warn!("garbage collection is disabled, remove is ignored");
        }
        Ok(())
    }

    #[inline]
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

    #[inline]
    fn extend<T: IntoIterator<Item = (Box<[u8]>, Self::Item)>>(
        &mut self,
        other: T,
    ) -> Result<(), Self::Error> {
        self.db.extend(other);
        Ok(())
    }
}
