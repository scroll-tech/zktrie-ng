//! Traits, helpers, and type definitions for databases.
//!
//! This module provides a trait for databases, as well as some
//! helper types and functions for working with databases.

use crate::db::kv::KVDatabaseItem;
use crate::hash::{HashScheme, ZkHash};
use crate::trie::{Node, NodeKind, NodeViewer};

/// key-value databases
pub mod kv;

/// A wrapper to store a trie node in the database.
#[derive(Debug)]
pub struct NodeDb<KvDb> {
    db: KvDb,
}

impl<KvDb: kv::KVDatabase> NodeDb<KvDb> {
    /// Create a new `NodeDb` with the given database.
    pub fn new(db: KvDb) -> Self {
        Self { db }
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

    /// Get a node from the database.
    pub fn get_node<H>(&self, hash: &ZkHash) -> Result<Option<NodeViewer>, KvDb::Error> {
        Ok(self.db.get(hash)?.map(|b| NodeViewer {
            data: b.into_bytes(),
            node_hash: *hash,
        }))
    }
}
