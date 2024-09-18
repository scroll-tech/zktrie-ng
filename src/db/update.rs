use crate::db::{HashMapDb, KVDatabase, SharedDb};

/// A zk database that can be updated.
///
/// Using a read-only database as source, and record all changes to another database.
///
/// ## Example
///
/// ```rust
/// use zktrie_ng::db::{HashMapDb, KVDatabase, SharedDb, UpdateDb};
///
/// let mut db = HashMapDb::default();
/// db.put(b"foo", b"bar").unwrap();
///
/// let mut shared_db = SharedDb::new(db); // it's now read-only
///
/// assert!(shared_db.put(b"foo", b"baz").is_err()); // can not write
///
/// let mut update_db = UpdateDb::new(HashMapDb::default(), shared_db);
///
/// assert_eq!(update_db.get(b"foo").unwrap().unwrap().as_ref(), b"bar");
///
/// update_db.put(b"foo", b"baz").unwrap();
///
/// assert_eq!(update_db.get(b"foo").unwrap().unwrap().as_ref(), b"baz");
/// ```
#[derive(Clone, Debug)]
pub struct UpdateDb<WriteDb = HashMapDb, CacheDb = SharedDb<SharedDb>> {
    write: WriteDb,
    cache: CacheDb,
}

impl<WriteDb: KVDatabase, CacheDb: KVDatabase> UpdateDb<WriteDb, CacheDb> {
    /// Create a new `UpdateDb` with the given write and cache databases.
    pub fn new(write: WriteDb, cache: CacheDb) -> Self {
        Self { write, cache }
    }
}

/// Error type for UpdateDb
#[derive(Debug, thiserror::Error)]
pub enum UpdateDbError<WriteDbErr, CacheDbErr> {
    /// Error when writing to the database
    #[error("write db error: {0}")]
    WriteDb(WriteDbErr),
    /// Error when reading from the cache database
    #[error("cache db error: {0}")]
    CacheDb(CacheDbErr),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum UpdateDbValue<W, C> {
    WriteDb(W),
    CacheDb(C),
}

impl<WriteDb: KVDatabase, CacheDb: KVDatabase> KVDatabase for UpdateDb<WriteDb, CacheDb> {
    type Error = UpdateDbError<WriteDb::Error, CacheDb::Error>;

    fn put_owned(
        &mut self,
        k: Box<[u8]>,
        v: Box<[u8]>,
    ) -> Result<Option<impl AsRef<[u8]>>, Self::Error> {
        self.write.put_owned(k, v).map_err(UpdateDbError::WriteDb)
    }

    fn get(&self, k: &[u8]) -> Result<Option<impl AsRef<[u8]>>, Self::Error> {
        if let Some(v) = self.write.get(k).map_err(UpdateDbError::WriteDb)? {
            return Ok(Some(UpdateDbValue::WriteDb(v)));
        }
        Ok(self
            .cache
            .get(k)
            .map_err(UpdateDbError::CacheDb)?
            .map(UpdateDbValue::CacheDb))
    }
}

impl<W: AsRef<[u8]>, C: AsRef<[u8]>> AsRef<[u8]> for UpdateDbValue<W, C> {
    fn as_ref(&self) -> &[u8] {
        match self {
            UpdateDbValue::WriteDb(w) => w.as_ref(),
            UpdateDbValue::CacheDb(c) => c.as_ref(),
        }
    }
}
