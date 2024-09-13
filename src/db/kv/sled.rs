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
use sled::Batch;

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

impl KVDatabase for SledDb {
    type Error = sled::Error;

    fn put(&mut self, k: &[u8], v: &[u8]) -> Result<Option<impl AsRef<[u8]>>, Self::Error> {
        self.db.insert(k, v)
    }

    fn put_owned(
        &mut self,
        k: Box<[u8]>,
        v: Box<[u8]>,
    ) -> Result<Option<impl AsRef<[u8]>>, Self::Error> {
        self.db.insert(k, v)
    }

    fn get(&self, k: &[u8]) -> Result<Option<impl AsRef<[u8]>>, Self::Error> {
        self.db.get(k)
    }

    fn set_gc_enabled(&mut self, gc_enabled: bool) {
        self.gc_enabled = gc_enabled;
    }

    fn gc_enabled(&self) -> bool {
        self.gc_enabled
    }

    fn remove(&mut self, k: &[u8]) -> Result<(), Self::Error> {
        if self.gc_enabled {
            self.db.remove(k)?;
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

    fn extend<T: IntoIterator<Item = (Box<[u8]>, Box<[u8]>)>>(
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
