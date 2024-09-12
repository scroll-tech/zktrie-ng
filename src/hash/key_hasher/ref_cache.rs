use crate::db::{HashMapDb, KVDatabase};
use crate::hash::{
    key_hasher::{KeyHasher, KeyHasherError},
    HashScheme, ZkHash,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Error type for [`RefCachedKeyHasher`]
#[derive(Debug, thiserror::Error)]
pub enum RefCachedKeyHasherErr<DbErr> {
    /// Error when read write db
    #[error("Db error: {0}")]
    Db(#[from] DbErr),
    /// Invalid hash read from db
    #[error("Invalid hash")]
    InvalidHash,
}

/// A !Send & !Sync hasher that cache the hash result into a db.
#[derive(Clone, Debug)]
pub struct RefCachedKeyHasher<H, Db = HashMapDb> {
    inner: Rc<RefCell<Db>>,
    _hash_scheme: std::marker::PhantomData<H>,
}

impl<H: HashScheme, Db: KVDatabase> RefCachedKeyHasher<H, Db> {
    /// Create a new RefCachedKeyHasher wrapping the given database.
    pub fn new(inner: Db) -> Self {
        Self {
            inner: Rc::new(RefCell::new(inner)),
            _hash_scheme: std::marker::PhantomData,
        }
    }

    /// Try to consume the RefCachedKeyHasher, returning the inner database.
    pub fn try_into_inner(self) -> Option<Db> {
        Rc::into_inner(self.inner).map(|db| db.into_inner())
    }

    /// Put a key-hash pair into the cache.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it does not check the validity of the hash.
    pub unsafe fn put_unchecked(&self, key: &[u8], hash: ZkHash) -> Result<(), Db::Error> {
        self.inner.borrow_mut().put(key, hash.as_ref())?;
        Ok(())
    }
}

impl<H: HashScheme, Db: KVDatabase> KeyHasher<H> for RefCachedKeyHasher<H, Db> {
    fn hash(&self, key: &[u8]) -> Result<ZkHash, KeyHasherError<H::Error>> {
        if let Some(hash) = self
            .inner
            .borrow_mut()
            .get(key)
            .map_err(RefCachedKeyHasherErr::Db)
            .map_err(|e| KeyHasherError::Other(Box::new(e)))?
        {
            let hash = hash.as_ref();
            let hash: &[u8; 32] = hash
                .try_into()
                .map_err(|_| RefCachedKeyHasherErr::<Db::Error>::InvalidHash)
                .map_err(|e| KeyHasherError::Other(Box::new(e)))?;
            return Ok(ZkHash::from(*hash));
        };
        let hash = H::hash_bytes(key).map_err(KeyHasherError::Hash)?;
        self.inner
            .borrow_mut()
            .put(key, hash.as_slice())
            .map_err(RefCachedKeyHasherErr::Db)
            .map_err(|e| KeyHasherError::Other(Box::new(e)))?;
        Ok(hash)
    }
}
