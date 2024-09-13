use crate::{
    db::{HashMapDb, KVDatabase},
    hash::{
        key_hasher::{KeyHasher, KeyHasherError, NoCacheHasher},
        poseidon::Poseidon,
        HashScheme, ZkHash, HASH_SIZE,
    },
    trie::{LazyNodeHash, Node, NodeType, ParseNodeError},
    HashMap, HashSet,
};
use std::error::Error;

mod imp;
#[cfg(test)]
mod tests;

/// A zkTrie implementation.
pub struct ZkTrie<H = Poseidon, Db = HashMapDb, K = NoCacheHasher> {
    db: Db,
    key_hasher: K,

    root: LazyNodeHash,
    dirty_branch_nodes: Vec<Node<H>>,
    dirty_leafs: HashMap<ZkHash, Node<H>>,
    gc_nodes: HashSet<LazyNodeHash>,

    _hash_scheme: std::marker::PhantomData<H>,
}

/// An iterator over the zkTrie.
pub struct ZkTrieIterator<'a, H, Db, K> {
    trie: &'a ZkTrie<H, Db, K>,
    stack: Vec<LazyNodeHash>,
}

/// Errors that can occur when using a zkTrie.
#[derive(Debug, thiserror::Error)]
pub enum ZkTrieError<HashErr, DbErr> {
    /// Error when hashing
    #[error(transparent)]
    Hash(HashErr),
    /// Error when accessing the database
    #[error("Database error: {0}")]
    Db(DbErr),
    /// Error when hashing the key
    #[error("Key hasher error: {0}")]
    KeyHasher(#[from] KeyHasherError<HashErr>),
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
    /// Expect a leaf node but got others
    #[error("Expect a leaf node but got others")]
    ExpectLeafNode,
    /// Unexpect value length
    #[error("Unexpect value length: expected {expected}, actual {actual}")]
    UnexpectValueLength {
        /// The expected length
        expected: usize,
        /// The actual length
        actual: usize,
    },
    /// Other errors
    #[error(transparent)]
    Other(Box<dyn Error>),
}
