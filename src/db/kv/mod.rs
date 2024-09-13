pub mod btree_map;
pub use btree_map::BTreeMapDb;

pub mod hash_map;
pub use hash_map::HashMapDb;

#[cfg(feature = "sled")]
#[cfg_attr(docsrs, doc(cfg(feature = "sled")))]
pub mod sled;
#[cfg(feature = "sled")]
#[cfg_attr(docsrs, doc(cfg(feature = "sled")))]
pub use sled::SledDb;

/// Store key-value pairs.
///
/// This trait is used to abstract over different key-value stores,
/// works likes a `HashMap<Box<[u8]>, Box<[u8]>>`.
pub trait KVDatabase {
    /// Associated error type.
    type Error: std::error::Error + 'static;

    /// Insert a key-value pair into the database.
    /// Returns the previous value associated with the key, if any.
    fn put(&mut self, k: &[u8], v: &[u8]) -> Result<Option<impl AsRef<[u8]>>, Self::Error> {
        self.put_owned(k.to_vec().into_boxed_slice(), v.to_vec().into_boxed_slice())
    }

    /// Insert an owned key-value pair into the database.
    /// Returns the previous value associated with the key, if any.
    fn put_owned(
        &mut self,
        k: Box<[u8]>,
        v: Box<[u8]>,
    ) -> Result<Option<impl AsRef<[u8]>>, Self::Error>;

    /// Retrieve the value associated with a key.
    /// Returns `Ok(None)` if the key is not present.
    fn get(&self, k: &[u8]) -> Result<Option<impl AsRef<[u8]>>, Self::Error>;

    /// Enable or disable the garbage collection support.
    ///
    /// # Returns
    ///
    /// The gc enabled state after the operation.
    ///
    /// i.e. If `set_gc_enabled(true)` returns `false`,
    /// it means the database does not support garbage collection,
    /// the setting is not changed.
    fn set_gc_enabled(&mut self, _gc_enabled: bool) -> bool {
        false
    }

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
    fn extend<T: IntoIterator<Item = (Box<[u8]>, Box<[u8]>)>>(
        &mut self,
        other: T,
    ) -> Result<(), Self::Error> {
        for (k, v) in other {
            self.put_owned(k, v)?;
        }
        Ok(())
    }
}
