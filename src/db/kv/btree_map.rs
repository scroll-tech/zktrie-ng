//! KVDatabase in-memory implementation using a [`BTreeMap`].
use super::KVDatabase;
use alloy_primitives::bytes::Bytes;
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt::Debug;

/// A simple in-memory key-value store backed by a [`BTreeMap`].
///
/// [`BTreeMap`] could be faster than [`HashMap`](std::collections::HashMap) in small size.
///
/// It's intended to be not [`Clone`], since [`Clone::clone`] will clone the entire [`BTreeMap`].
///
/// If you need to clone the entire database,
/// you can use [`BTreeMapDb::inner`] to get the inner [`BTreeMap`],
/// and then clone the [`BTreeMap`] manually and create a new via [`BTreeMapDb::from_map`].
#[derive(Default)]
pub struct BTreeMapDb {
    gc_enabled: bool,
    db: BTreeMap<Box<[u8]>, Bytes>,
}

impl BTreeMapDb {
    /// Create a new empty `BTreeMapDb`.
    pub fn new(gc_enabled: bool) -> Self {
        Self {
            gc_enabled,
            db: BTreeMap::new(),
        }
    }

    /// Create a new `BTreeMapDb` from a `BTreeMap`.
    pub fn from_map(gc_enabled: bool, db: BTreeMap<Box<[u8]>, Bytes>) -> Self {
        Self { gc_enabled, db }
    }

    /// Get the inner `BTreeMap`.
    pub fn inner(&self) -> &BTreeMap<Box<[u8]>, Bytes> {
        &self.db
    }

    /// Into the inner `BTreeMap`.
    pub fn into_inner(self) -> BTreeMap<Box<[u8]>, Bytes> {
        self.db
    }
}

impl Debug for BTreeMapDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BTreeMapDb").field(&self.db.len()).finish()
    }
}

impl KVDatabase for BTreeMapDb {
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
