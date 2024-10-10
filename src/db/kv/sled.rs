//! [`KVDatabase`] implementation using [`sled`](https://docs.rs/sled/latest/sled/).
//!
//! Different from [`HashMapDb`](crate::db::HashMapDb) and [`BTreeMapDb`](crate::db::BTreeMapDb),
//! [`SledDb`] is `Clone`, since [`sled::Tree`] is `Clone`.
//! Same db is shared between different instances of [`SledDb`].
//!
//! ## Example
//!
//! ```rust
//! use zktrie_ng::{
//!     trie,
//!     hash::{
//!         key_hasher::NoCacheHasher,
//!         poseidon::Poseidon,
//!     },
//!     db::SledDb,
//! };
//!
//! // A ZkTrie using Poseidon hash scheme,
//! // sled as backend kv database and NoCacheHasher as key hasher.
//! type ZkTrie = trie::ZkTrie<Poseidon, SledDb, NoCacheHasher>;
//!
//! let db = sled::open("my_db").unwrap();
//! let tree = db.open_tree("zk_trie").unwrap();
//!
//! let mut trie = ZkTrie::new(SledDb::new(true, tree), NoCacheHasher);
//! ```

use super::KVDatabase;
use crate::db::KVDatabaseItem;
use alloy_primitives::bytes::Bytes;
use sled::{Batch, IVec};

/// A key-value store backed by [`sled`].
#[derive(Clone, Debug)]
pub struct SledDb {
    gc_enabled: bool,
    db: sled::Tree,
}

impl SledDb {
    /// Create a new `SledDb` wrapping the given `sled::Tree`.
    pub fn new(gc_enabled: bool, db: sled::Tree) -> Self {
        Self { gc_enabled, db }
    }

    /// Get the inner [`sled::Tree`]
    pub fn inner(&self) -> &sled::Tree {
        &self.db
    }

    /// Into the inner [`sled::Tree`]
    pub fn into_inner(self) -> sled::Tree {
        self.db
    }
}

impl KVDatabaseItem for IVec {
    #[inline]
    fn from_slice(value: &[u8]) -> Self {
        IVec::from(value)
    }

    #[inline]
    fn from_bytes(bytes: Bytes) -> Self {
        IVec::from(bytes.to_vec())
    }

    #[inline]
    fn into_bytes(self) -> Bytes {
        Bytes::from(self.to_vec())
    }
}

impl KVDatabase for SledDb {
    type Item = IVec;

    type Error = sled::Error;

    #[inline]
    fn contains_key(&self, k: &[u8]) -> Result<bool, Self::Error> {
        self.db.contains_key(k)
    }

    #[inline]
    fn put(&mut self, k: &[u8], v: &[u8]) -> Result<Option<Self::Item>, Self::Error> {
        self.db.insert(k, v)
    }

    #[inline]
    fn put_owned<K: AsRef<[u8]> + Into<Box<[u8]>>>(
        &mut self,
        k: K,
        v: impl Into<Self::Item>,
    ) -> Result<Option<Self::Item>, Self::Error> {
        self.db.insert(k.as_ref(), v)
    }

    #[inline]
    fn get<K: AsRef<[u8]> + Clone>(&self, k: K) -> Result<Option<Self::Item>, Self::Error> {
        self.db.get(k)
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
            self.db.remove(k)?;
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
        let mut batch = Batch::default();
        for entry in self.db.iter() {
            let (k, v) = entry?;
            if !f(k.as_ref(), v.as_ref()) {
                batch.remove(k);
                removed += 1;
            }
        }
        trace!("{} key-value pairs removed", removed);
        self.db.apply_batch(batch)
    }

    #[inline]
    fn extend<T: IntoIterator<Item = (Box<[u8]>, Self::Item)>>(
        &mut self,
        other: T,
    ) -> Result<(), Self::Error> {
        let mut batch = Batch::default();
        for (k, v) in other {
            batch.insert(k, v);
        }
        self.db.apply_batch(batch)
    }
}
