use alloy_primitives::bytes::Bytes;

mod extend;

pub mod btree_map;
pub use btree_map::BTreeMapDb;

pub mod hash_map;
pub use hash_map::HashMapDb;

pub mod middleware;

#[cfg(feature = "sled")]
#[cfg_attr(docsrs, doc(cfg(feature = "sled")))]
pub mod sled;
#[cfg(feature = "sled")]
#[cfg_attr(docsrs, doc(cfg(feature = "sled")))]
pub use sled::SledDb;

/// Necessary trait for values stored in a key-value database.
pub trait KVDatabaseItem: From<Vec<u8>> + From<Bytes> + AsRef<[u8]> + Clone {
    /// Construct a value from a slice.
    fn from_slice(value: &[u8]) -> Self {
        value.to_vec().into()
    }

    /// Turn the value into a [`Bytes`].
    fn into_bytes(self) -> Bytes;
}

/// Store key-value pairs.
///
/// This trait is used to abstract over different key-value stores,
/// works likes a `HashMap<Box<[u8]>, Box<[u8]>>`.
pub trait KVDatabase {
    /// Value type returned by the database.
    type Item: KVDatabaseItem;

    /// Associated error type.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Check if the database contains a key.
    fn contains_key(&self, k: &[u8]) -> Result<bool, Self::Error> {
        Ok(self.get(k)?.is_some())
    }

    /// Insert a key-value pair into the database.
    /// Returns the previous value associated with the key, if any.
    fn put(&mut self, k: &[u8], v: &[u8]) -> Result<Option<Self::Item>, Self::Error>;

    /// Insert a key-value pair if the key is not present.
    fn or_put(&mut self, k: &[u8], v: &[u8]) -> Result<(), Self::Error> {
        if !self.contains_key(k)? {
            self.put(k, v)?;
        }

        Ok(())
    }

    /// Insert a key-value pair from a closure if the key is not present.
    fn or_put_with<O: Into<Self::Item>, F: FnOnce() -> O>(
        &mut self,
        k: &[u8],
        default: F,
    ) -> Result<(), Self::Error> {
        if !self.contains_key(k)? {
            self.put_owned(k, default().into())?;
        }
        Ok(())
    }

    /// Insert an owned key-value pair into the database.
    /// Returns the previous value associated with the key, if any.
    fn put_owned<K: AsRef<[u8]> + Into<Box<[u8]>>>(
        &mut self,
        k: K,
        v: impl Into<Self::Item>,
    ) -> Result<Option<Self::Item>, Self::Error>;

    /// Retrieve the value associated with a key.
    /// Returns `Ok(None)` if the key is not present.
    fn get<K: AsRef<[u8]> + Clone>(&self, k: K) -> Result<Option<Self::Item>, Self::Error>;

    /// Check if the database supports garbage collection.
    fn is_gc_supported(&self) -> bool {
        false
    }

    /// Enable or disable the garbage collection support.
    fn set_gc_enabled(&mut self, _gc_enabled: bool) {}

    /// Check if garbage collection is enabled.
    fn gc_enabled(&self) -> bool {
        false
    }

    /// Best-effort removal of a key-value pair from the database, used for garbage collection.
    ///
    /// # Note
    ///
    /// For implementations that do not support removal, this method should not be overridden.
    ///
    /// If `Ok(())` returns, the removal may be:
    /// - removed (the key was present and removed or the key was not present),
    /// - unsupported (the database does not support removal).
    /// - planned, but not yet executed (i.e. the database is busy and the operation is queued).
    ///
    /// If `Err(e)` returns, it can only be: the database supports removal but the operation fails.
    ///
    /// You shall **NEVER** rely on the return value to determine if the key was present or not.
    fn remove(&mut self, _k: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Retain only the key-value pairs that satisfy the predicate.
    ///
    /// # Note
    ///
    /// Same as [`KVDatabase::remove`], this method is best-effort and should not be relied on
    /// to determine if the key was present or not.
    fn retain<F>(&mut self, _f: F) -> Result<(), Self::Error>
    where
        F: FnMut(&[u8], &[u8]) -> bool,
    {
        Ok(())
    }

    /// Extend the database with the key-value pairs from the iterator.
    fn extend<T: IntoIterator<Item = (Box<[u8]>, Self::Item)>>(
        &mut self,
        other: T,
    ) -> Result<(), Self::Error> {
        for (k, v) in other {
            self.put_owned(k, v)?;
        }
        Ok(())
    }
}

impl KVDatabaseItem for Bytes {
    #[inline]
    fn into_bytes(self) -> Bytes {
        self
    }
}
