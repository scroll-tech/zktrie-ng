use crate::db::kv::{HashMapDb, KVDatabase};
use crate::hash::{
    key_hasher::{KeyHasher, KeyHasherError},
    HashScheme, ZkHash,
};
use std::sync::{Arc, Mutex};

/// Error type for [`SyncCachedKeyHasher`]
#[derive(Debug, thiserror::Error)]
pub enum SyncCachedKeyHasherErr<DbErr> {
    /// Error when read write db
    #[error("Db error: {0}")]
    Db(#[from] DbErr),
    /// Invalid hash read from db
    #[error("Invalid hash")]
    InvalidHash,
}

/// A Send & Sync hasher that cache the hash result into a db.
#[derive(Clone, Debug)]
pub struct SyncCachedKeyHasher<H, Db = HashMapDb> {
    inner: Arc<Mutex<Db>>,
    _hash_scheme: std::marker::PhantomData<H>,
}

impl<H: HashScheme, Db: KVDatabase> SyncCachedKeyHasher<H, Db> {
    /// Create a new KeyCacheDb wrapping the given database.
    pub fn new(inner: Db) -> Self {
        Self {
            inner: Arc::new(Mutex::new(inner)),
            _hash_scheme: std::marker::PhantomData,
        }
    }

    /// Try to consume the KeyCacheDb, returning the inner database.
    pub fn try_into_inner(self) -> Option<Db> {
        Arc::into_inner(self.inner).and_then(|db| db.into_inner().ok())
    }

    /// Put a key-hash pair into the cache.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it does not check the validity of the hash.
    pub unsafe fn put_unchecked(&self, key: &[u8], hash: ZkHash) -> Result<(), Db::Error> {
        self.inner.lock().unwrap().put(key, hash.as_ref())?;
        Ok(())
    }
}

impl<H: HashScheme, Db: KVDatabase> KeyHasher<H> for SyncCachedKeyHasher<H, Db> {
    fn hash(&self, key: &[u8]) -> Result<ZkHash, KeyHasherError<H::Error>> {
        let mut db = self.inner.lock().unwrap();
        if let Some(hash) = db
            .get(key)
            .map_err(SyncCachedKeyHasherErr::Db)
            .map_err(|e| KeyHasherError::Other(Box::new(e)))?
        {
            let hash = hash.as_ref();
            let hash: &[u8; 32] = hash
                .try_into()
                .map_err(|_| SyncCachedKeyHasherErr::<Db::Error>::InvalidHash)
                .map_err(|e| KeyHasherError::Other(Box::new(e)))?;
            return Ok(ZkHash::from(*hash));
        };
        let hash = H::hash_bytes(key).map_err(KeyHasherError::Hash)?;
        db.put(key, hash.as_slice())
            .map_err(SyncCachedKeyHasherErr::Db)
            .map_err(|e| KeyHasherError::Other(Box::new(e)))?;
        Ok(hash)
    }
}
