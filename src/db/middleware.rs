//! Middleware for kv database.
use crate::db::{KVDatabase, KVDatabaseItem};
use crate::HashMap;
use alloy_primitives::bytes::Bytes;
use std::mem;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

/// A middleware that records all read items.
pub struct RecorderMiddleware<Db> {
    inner: Db,
    read_items: Arc<Mutex<HashMap<Vec<u8>, Bytes>>>,
}

impl<Db> RecorderMiddleware<Db> {
    /// Create a new `RecorderMiddleware` wrapping the given database.
    pub fn new(inner: Db) -> Self {
        Self {
            inner,
            read_items: Arc::default(),
        }
    }

    /// Take the recorded items and leave an empty map.
    #[inline]
    pub fn take_read_items(&self) -> HashMap<Vec<u8>, Bytes> {
        mem::take(self.read_items.lock().unwrap().deref_mut())
    }

    /// Into the inner database.
    pub fn into_inner(self) -> Db {
        self.inner
    }
}

impl<Db: Clone> Clone for RecorderMiddleware<Db> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            read_items: Arc::clone(&self.read_items),
        }
    }
}

impl<Db: KVDatabase> KVDatabase for RecorderMiddleware<Db> {
    type Item = Db::Item;
    type Error = Db::Error;

    fn contains_key(&self, k: &[u8]) -> Result<bool, Self::Error> {
        self.inner.contains_key(k)
    }

    fn put(&mut self, k: &[u8], v: &[u8]) -> Result<Option<Self::Item>, Self::Error> {
        self.inner.put(k, v)
    }

    fn or_put(&mut self, k: &[u8], v: &[u8]) -> Result<(), Self::Error> {
        self.inner.or_put(k, v)
    }

    fn or_put_with<O: Into<Self::Item>, F: FnOnce() -> O>(
        &mut self,
        k: &[u8],
        default: F,
    ) -> Result<(), Self::Error> {
        self.inner.or_put_with(k, default)
    }

    fn put_owned<K: AsRef<[u8]> + Into<Box<[u8]>>>(
        &mut self,
        k: K,
        v: impl Into<Self::Item>,
    ) -> Result<Option<Self::Item>, Self::Error> {
        self.inner.put_owned(k, v)
    }

    fn get<K: AsRef<[u8]> + Clone>(&self, k: K) -> Result<Option<Self::Item>, Self::Error> {
        let result = self.inner.get(k.clone().as_ref())?;
        if let Some(value) = &result {
            self.read_items
                .lock()
                .unwrap()
                .insert(k.as_ref().to_vec(), value.clone().into_bytes());
        }
        Ok(result)
    }

    #[inline(always)]
    fn is_gc_supported(&self) -> bool {
        self.inner.is_gc_supported()
    }

    #[inline(always)]
    fn set_gc_enabled(&mut self, gc_enabled: bool) {
        self.inner.set_gc_enabled(gc_enabled)
    }

    #[inline(always)]
    fn gc_enabled(&self) -> bool {
        self.inner.gc_enabled()
    }

    #[inline(always)]
    fn remove(&mut self, k: &[u8]) -> Result<(), Self::Error> {
        self.inner.remove(k)
    }

    #[inline(always)]
    fn retain<F>(&mut self, f: F) -> Result<(), Self::Error>
    where
        F: FnMut(&[u8], &[u8]) -> bool,
    {
        self.inner.retain(f)
    }

    #[inline(always)]
    fn extend<T: IntoIterator<Item = (Box<[u8]>, Self::Item)>>(
        &mut self,
        other: T,
    ) -> Result<(), Self::Error> {
        self.inner.extend(other)
    }
}
