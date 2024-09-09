use crate::db::{HashMapDb, KVDatabase, KeyCacheDb};

mod node;
pub use node::*;

pub struct ZkTrie<const MAX_LEVELS: usize, DB = HashMapDb, CacheDb = HashMapDb> {
    db: DB,
    key_cache: KeyCacheDb<CacheDb>,
}
