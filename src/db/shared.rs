use crate::db::{HashMapDb, KVDatabase};
use std::sync::Arc;

/// Readonly version of the database, could be shared.
#[derive(Clone, Debug)]
pub struct SharedDb<Db = HashMapDb>(Arc<Db>);

/// Error type for SharedZkDatabase
#[derive(Debug, thiserror::Error)]
pub enum SharedZkDatabaseError<E> {
    /// Try to write to a shared read-only database
    #[error("SharedZkDatabaseError is read-only")]
    ReadOnly,
    /// Error when accessing the database
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
