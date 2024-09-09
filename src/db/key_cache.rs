use crate::db::{HashMapDb, KVDatabase};

/// KeyCacheDb is a wrapper around a KVDatabase that caches key hashes in memory.
#[derive(Clone)]
pub struct KeyCacheDb<Db: KVDatabase, CacheDb: KVDatabase = HashMapDb> {
    db: Db,
    key_cache: CacheDb,
}