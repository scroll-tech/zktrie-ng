mod btree_map;
pub use btree_map::BTreeMapDb;

mod hash_map;
pub use hash_map::HashMapDb;

#[cfg(feature = "sled")]
#[cfg_attr(docsrs, doc(cfg(feature = "sled")))]
mod sled;
#[cfg(feature = "sled")]
#[cfg_attr(docsrs, doc(cfg(feature = "sled")))]
pub use sled::SledDb;

/// Store key-value pairs.
///
/// This trait is used to abstract over different key-value stores,
/// works likes a `HashMap<Box<[u8]>, Box<[u8]>>`.
pub trait KVDatabase: Clone {
    type Error;

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
