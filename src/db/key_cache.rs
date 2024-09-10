use crate::{
    db::{HashMapDb, KVDatabase},
    hash::{HashScheme, ZkHash},
};

/// A cache db that stores the hash of a key.
#[derive(Clone)]
pub struct KeyCacheDb<H, Db = HashMapDb> {
    inner: Db,
    _hash_scheme: std::marker::PhantomData<H>,
}

/// Error type for KeyCacheDb
#[derive(Debug, thiserror::Error)]
pub enum KeyCacheError<HashErr, DbErr> {
    #[error(transparent)]
    HashError(HashErr),
    #[error(transparent)]
    DbError(DbErr),
    #[error("Invalid hash")]
    InvalidHash,
}

impl<H: HashScheme, Db: KVDatabase> KeyCacheDb<H, Db> {
    /// Create a new KeyCacheDb wrapping the given database.
    pub fn new(inner: Db) -> Self {
        Self {
            inner,
            _hash_scheme: std::marker::PhantomData,
        }
    }

    /// Unwrap the KeyCacheDb, returning the inner database.
    pub fn into_inner(self) -> Db {
        self.inner
    }

    /// Get the hash of a key from the cache, if it is present.
    pub fn get(&self, key: &[u8]) -> Result<Option<ZkHash>, KeyCacheError<H::Error, Db::Error>> {
        if let Some(hash) = self.inner.get(key).map_err(KeyCacheError::DbError)? {
            let hash = hash.as_ref();
            let hash: &[u8; 32] = hash.try_into().map_err(|_| KeyCacheError::InvalidHash)?;
            return Ok(Some(ZkHash::from(*hash)));
        };
        Ok(None)
    }

    /// Get the hash of a key from the cache,
    /// or compute it and store it in the cache if it is not present.
    pub fn get_or_compute_if_absent(
        &mut self,
        key: &[u8],
    ) -> Result<ZkHash, KeyCacheError<H::Error, Db::Error>> {
        if let Some(hash) = self.get(key)? {
            return Ok(hash);
        }
        let hash = H::hash_bytes(key).map_err(KeyCacheError::HashError)?;
        self.inner
            .put(key, hash.as_slice())
            .map_err(KeyCacheError::DbError)?;
        Ok(hash)
    }

    /// Put a key-hash pair into the cache.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it does not check the validity of the hash.
    pub unsafe fn put_unchecked(
        &mut self,
        key: &[u8],
        hash: ZkHash,
    ) -> Result<(), KeyCacheError<H::Error, Db::Error>> {
        self.inner
            .put(key, hash.as_ref())
            .map_err(KeyCacheError::DbError)?;
        Ok(())
    }
}
