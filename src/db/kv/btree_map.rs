use super::KVDatabase;
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt::Debug;

/// A simple in-memory key-value store backed by a `BTreeMap`.
#[derive(Clone, Default)]
pub struct BTreeMapDb {
    db: BTreeMap<Box<[u8]>, Box<[u8]>>,
}

impl BTreeMapDb {
    /// Create a new empty `BTreeMapDb`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Debug for BTreeMapDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BTreeMapDb").field(&self.db.len()).finish()
    }
}

impl KVDatabase for BTreeMapDb {
    type Error = Infallible;

    fn put_owned(
        &mut self,
        k: Box<[u8]>,
        v: Box<[u8]>,
    ) -> Result<Option<impl AsRef<[u8]>>, Self::Error> {
        Ok(self.db.insert(k, v))
    }

    fn get(&self, k: &[u8]) -> Result<Option<impl AsRef<[u8]>>, Self::Error> {
        Ok(self.db.get(k).map(|v| v.as_ref()))
    }

    fn extend<T: IntoIterator<Item = (Box<[u8]>, Box<[u8]>)>>(
        &mut self,
        other: T,
    ) -> Result<(), Self::Error> {
        self.db.extend(other);
        Ok(())
    }
}
