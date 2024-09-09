use super::KVDatabase;
use std::convert::Infallible;

#[derive(Clone, Default)]
pub struct HashMapDb {
    db: crate::HashMap<Box<[u8]>, Box<[u8]>>,
}

impl HashMapDb {
    pub fn new() -> Self {
        Self::default()
    }
}

impl KVDatabase for HashMapDb {
    type Error = Infallible;

    fn put_owned(
        &mut self,
        k: Box<[u8]>,
        v: Box<[u8]>,
    ) -> Result<Option<Box<[u8]>>, Self::Error> {
        Ok(self.db.insert(k, v))
    }

    fn get(&self, k: &[u8]) -> Result<Option<&[u8]>, Self::Error> {
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
