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
//! let mut trie = ZkTrie::new(SledDb::new(tree), NoCacheHasher);
//! ```
use super::KVDatabase;
use sled::Batch;

/// A key-value store backed by [`sled`].
#[derive(Clone, Debug)]
pub struct SledDb {
    db: sled::Tree,
}

impl SledDb {
    /// Create a new `SledDb` wrapping the given `sled::Tree`.
    pub fn new(db: sled::Tree) -> Self {
        Self { db }
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

    fn remove(&mut self, k: &[u8]) -> Result<(), Self::Error> {
        self.db.remove(k)?;
        Ok(())
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
