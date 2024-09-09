use crate::db::{HashMapDb, KVDatabase};
use std::rc::Rc;

/// Readonly version of the database, could be shared.
#[derive(Clone)]
pub struct SharedDb<Db: KVDatabase = HashMapDb>(Rc<Db>);

/// Error type for SharedZkDatabase
#[derive(Debug, thiserror::Error)]
pub enum SharedZkDatabaseError<E> {
    #[error("SharedZkDatabaseError is read-only")]
    ReadOnly,
    #[error(transparent)]
    Inner(E),
}

impl<Db: KVDatabase> KVDatabase for SharedDb<Db> {
    type Error = SharedZkDatabaseError<Db::Error>;

    fn put_owned(
        &mut self,
        _k: Box<[u8]>,
        _v: Box<[u8]>,
    ) -> Result<Option<impl AsRef<[u8]>>, Self::Error> {
        Err::<Option<&[u8]>, _>(SharedZkDatabaseError::ReadOnly)
    }

    fn get(&self, k: &[u8]) -> Result<Option<impl AsRef<[u8]>>, Self::Error> {
        self.0.get(k).map_err(SharedZkDatabaseError::Inner)
    }
}
