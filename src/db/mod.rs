//! Traits, helpers, and type definitions for databases.
//!
//! This module provides a trait for databases, as well as some
//! helper types and functions for working with databases.

use crate::db::kv::{HashMapDb, KVDatabase, KVDatabaseItem};
use crate::hash::{HashScheme, ZkHash};
use crate::trie::{Node, NodeKind, NodeViewer};
use std::fmt::Debug;

/// key-value databases
pub mod kv;

/// A wrapper to store a trie node in the database.
pub struct NodeDb<KvDb> {
    db: KvDb,
}

impl Default for NodeDb<HashMapDb> {
    fn default() -> Self {
        Self::new(HashMapDb::default())
    }
}

impl<KvDb: KVDatabase> NodeDb<KvDb> {
    /// Create a new `NodeDb` with the given database.
    #[inline]
    pub fn new(db: KvDb) -> Self {
        Self { db }
    }

    /// Get inner db
    pub fn inner(&self) -> &KvDb {
        &self.db
    }

    /// Into inner db
    pub fn into_inner(self) -> KvDb {
        self.db
    }

    /// Check if the database supports garbage collection.
    #[inline]
    pub fn is_gc_supported(&self) -> bool {
        self.db.is_gc_supported()
    }

    /// Enable or disable the garbage collection support.
    #[inline]
    pub fn set_gc_enabled(&mut self, gc_enabled: bool) {
        self.db.set_gc_enabled(gc_enabled);
    }

    /// Check if garbage collection is enabled.
    #[inline]
    pub fn gc_enabled(&self) -> bool {
        self.db.gc_enabled()
    }

    /// Put a node into the database.
    pub fn put_node<H: HashScheme>(&mut self, node: &Node<H>) -> Result<(), KvDb::Error> {
        let node_hash = node.node_hash.get().expect("Node hash not calculated");
        if let NodeKind::Branch(branch) = node.data.as_ref() {
            if !branch.child_right().is_resolved() || !branch.child_left().is_resolved() {
                panic!("Cannot archive branch node with unresolved child hash");
            }
        }
        let bytes = rkyv::to_bytes::<_, 1024>(node).expect("infallible");
        self.db.put(node_hash.as_ref(), bytes.as_ref())?;
        Ok(())
    }

    /// Put a archived node bytes into the database.
    pub unsafe fn put_archived_node_unchecked(
        &mut self,
        node_hash: ZkHash,
        bytes: Bytes,
    ) -> Result<(), KvDb::Error> {
        self.db.put_owned(node_hash, bytes)
    }

    /// Get a node from the database.
    pub fn get_node<H>(&self, hash: &ZkHash) -> Result<Option<NodeViewer>, KvDb::Error> {
        Ok(self.db.get(hash)?.map(|b| NodeViewer {
            data: b.into_bytes(),
            node_hash: *hash,
        }))
    }

    /// Removes a node from the database.
    ///
    /// # Note
    ///
    /// See also [`KVDatabase::remove`].
    pub fn remove_node(&mut self, hash: &ZkHash) -> Result<(), KvDb::Error> {
        self.db.remove(hash.as_ref())
    }

    /// Retain only the nodes that satisfy the predicate.
    ///
    /// # Note
    ///
    /// See also [`KVDatabase::retain`].
    pub fn retain<F>(&mut self, mut f: F) -> Result<(), KvDb::Error>
    where
        F: FnMut(&ZkHash) -> bool,
    {
        self.db.retain(|k, _| f(&ZkHash::from_slice(k)))
    }
}

impl<KvDb: Debug> Debug for NodeDb<KvDb> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeDb").field("db", &self.db).finish()
    }
}

impl<KvDb: Clone> Clone for NodeDb<KvDb> {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
        }
    }
}
