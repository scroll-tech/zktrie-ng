use crate::{
    db::{HashMapDb, KVDatabase, KeyCacheDb, KeyCacheError},
    hash::{poseidon::Poseidon, HashScheme, ZkHash, HASH_SIZE},
    trie::{LazyNodeHash, Node, NodeType, ParseNodeError},
    HashMap,
};

mod imp;
#[cfg(test)]
mod tests;

/// A zkTrie implementation.
pub struct ZkTrie<const MAX_LEVEL: usize, H = Poseidon, Db = HashMapDb, CacheDb = HashMapDb> {
    db: Db,
    key_cache: KeyCacheDb<H, CacheDb>,

    root: LazyNodeHash,
    dirty_branch_nodes: Vec<Node<H>>,
    dirty_leafs: HashMap<ZkHash, Node<H>>,

    _hash_scheme: std::marker::PhantomData<H>,
}

/// Errors that can occur when using a zkTrie.
#[derive(Debug, thiserror::Error)]
pub enum ZkTrieError<HashErr, DbErr, CacheDbErr> {
    /// Error when hashing
    #[error(transparent)]
    Hash(HashErr),
    /// Error when accessing the database
    #[error("Database error: {0}")]
    Db(DbErr),
    /// Error when accessing the cache database
    #[error("Key cache error: {0}")]
    KeyCache(#[from] KeyCacheError<HashErr, CacheDbErr>),
    /// Error when parsing a node
    #[error("Invalid node bytes: {0}")]
    InvalidNodeBytes(#[from] ParseNodeError<HashErr>),
    /// Error when trying to use an unresolved hash
    #[error("Trying to use unresolved hash")]
    UnresolvedHashUsed,
    /// Error when a node is not found
    #[error("Node not found")]
    NodeNotFound,
    /// Error when the max level is reached
    #[error("Max level reached")]
    MaxLevelReached,
}
