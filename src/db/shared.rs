use crate::db::{HashMapDb, KVDatabase};
use std::sync::Arc;

/// Readonly version of the database, could be shared.
///
/// ## Example
///
/// ```rust
/// use zktrie_ng::db::{HashMapDb, KVDatabase, SharedDb};
///
/// let mut db = HashMapDb::default();
/// db.put(b"foo", b"bar").unwrap();
///
/// // db.clone() <- can not compile
///
/// let shared_db = SharedDb::new(db);
///
/// let _shared_trie = shared_db.clone();
///
/// assert_eq!(shared_db.get(b"foo").unwrap().unwrap().as_ref(), b"bar");
/// ```
#[derive(Debug)]
pub struct SharedDb<Db = HashMapDb>(Arc<Db>);

impl<Db: KVDatabase> SharedDb<Db> {
    /// Create a new shared database
    pub fn new(db: Db) -> Self {
        Self(Arc::new(db))
    }
}

impl<Db> Clone for SharedDb<Db> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// Error type for SharedZkDatabase
#[derive(Debug, thiserror::Error)]
pub enum SharedZkDatabaseError<E> {
    /// Try to write to a shared read-only database
    #[error("SharedZkDatabaseError is read-only")]
    ReadOnly,
    /// Error when accessing the database
    #[error(transparent)]
    Inner(E),
}

impl<Db: KVDatabase> KVDatabase for SharedDb<Db> {
    type Error = SharedZkDatabaseError<Db::Error>;

    fn put_owned(
        &mut self,
        _k: Box<[u8]>,
        _v: Box<[u8]>,
    ) -> Result<Option<impl AsRef<[u8]>>, Self::Error> {
        Err::<Option<&[u8]>, _>(SharedZkDatabaseError::ReadOnly)
    }

    fn get(&self, k: &[u8]) -> Result<Option<impl AsRef<[u8]>>, Self::Error> {
        self.0.get(k).map_err(SharedZkDatabaseError::Inner)
    }
}
