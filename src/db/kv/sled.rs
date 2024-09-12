use super::KVDatabase;
use sled::Batch;

/// A key-value store backed by `sled`.
#[derive(Clone)]
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
