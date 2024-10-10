use super::*;

use crate::trie::INode;
use crate::{
    db::kv::KVDatabase,
    trie::{DecodeValueBytes, EncodeValueBytes, LazyBranchHash, MAGIC_NODE_BYTES},
};
use std::fmt::{Debug, Formatter};

type Result<T, H, DB> =
    std::result::Result<T, ZkTrieError<<H as HashScheme>::Error, <DB as KVDatabase>::Error>>;

impl Default for ZkTrie {
    fn default() -> Self {
        Self::new(NoCacheHasher)
    }
}

impl<H: HashScheme, K: KeyHasher<H>> Debug for ZkTrie<H, K> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ZkTrie")
            .field("MAX_LEVEL", &H::TRIE_MAX_LEVELS)
            .field("hash_scheme", &std::any::type_name::<H>())
            .field("root", &self.root)
            .field("is_dirty", &self.is_dirty())
            .finish()
    }
}

impl<H: HashScheme, K: KeyHasher<H>> ZkTrie<H, K> {
    /// Create a new zkTrie
    #[inline(always)]
    pub fn new(key_hasher: K) -> Self {
        Self {
            key_hasher,
            root: ZkHash::default().into(),
            dirty_branch_nodes: Vec::new(),
            dirty_leafs: HashMap::new(),
            gc_nodes: HashSet::new(),
            _hash_scheme: std::marker::PhantomData,
        }
    }

    /// Create a new zkTrie with a given root hash
    #[inline]
    pub fn new_with_root<Db: KVDatabase>(
        db: &NodeDb<Db>,
        key_hasher: K,
        root: ZkHash,
    ) -> Result<Self, H, Db> {
        let this = Self {
            key_hasher,
            root: root.into(),
            dirty_branch_nodes: Vec::new(),
            dirty_leafs: HashMap::new(),
            gc_nodes: HashSet::new(),
            _hash_scheme: std::marker::PhantomData,
        };

        this.get_node_by_hash(db, root)?;

        Ok(this)
    }

    /// Get the underlying key hasher
    #[inline(always)]
    pub fn key_hasher(&self) -> &K {
        &self.key_hasher
    }

    /// Check if the trie is dirty
    #[inline(always)]
    pub fn is_dirty(&self) -> bool {
        !self.dirty_branch_nodes.is_empty() || !self.dirty_leafs.is_empty()
    }

    /// Get the root hash of the trie, may be unresolved if the trie is dirty
    #[inline(always)]
    pub fn root(&self) -> &LazyNodeHash {
        &self.root
    }

    /// Get a value from the trie, which can be decoded from bytes
    ///
    /// # Returns
    ///
    /// - `Ok(Some(value))` if the key is found
    /// - `Ok(None)` if the key is not found
    /// - `Err(e)` if other error occurs
    #[instrument(level = "trace", skip_all)]
    pub fn get<Db: KVDatabase, T: DecodeValueBytes, KEY: AsRef<[u8]>>(
        &self,
        db: &NodeDb<Db>,
        key: KEY,
    ) -> Result<Option<T>, H, Db> {
        let key = key.as_ref();
        trace!(key = hex::encode(key));
        let node_key = self.key_hasher.hash(key)?;
        trace!(node_key = ?node_key);
        let node = self.get_node_by_key(db, &node_key)?;
        match node.node_type() {
            NodeType::Empty => Ok(None),
            NodeType::Leaf => {
                let leaf = node.as_leaf().unwrap();
                let values = leaf.value_preimages();

                if let Some(t) = T::decode_values_bytes(values) {
                    Ok(Some(t))
                } else {
                    Err(ZkTrieError::UnexpectValue)
                }
            }
            _ => Err(ZkTrieError::ExpectLeafNode),
        }
    }

    /// Update the trie with a new key-value pair, which value can be encoded to bytes
    #[inline(always)]
    #[instrument(level = "trace", skip_all)]
    pub fn update<Db: KVDatabase, T: EncodeValueBytes, KEY: AsRef<[u8]>>(
        &mut self,
        db: &NodeDb<Db>,
        key: KEY,
        value: T,
    ) -> Result<(), H, Db> {
        let (values, compression_flags) = value.encode_values_bytes();
        self.raw_update(db, key, values, compression_flags)
    }

    /// Update the trie with a new key-values pair
    #[instrument(level = "trace", skip_all)]
    pub fn raw_update<Db: KVDatabase, KEY: AsRef<[u8]>>(
        &mut self,
        db: &NodeDb<Db>,
        key: KEY,
        value_preimages: Vec<[u8; 32]>,
        compression_flags: u32,
    ) -> Result<(), H, Db> {
        let key = key.as_ref();
        trace!(key = hex::encode(key));
        let node_key = self.key_hasher.hash(key)?;
        trace!(node_key = ?node_key);
        let new_leaf = Node::new_leaf(node_key, value_preimages, compression_flags, None)
            .map_err(ZkTrieError::Hash)?;
        self.root = self.add_leaf(db, new_leaf, self.root.clone(), 0)?.0;
        Ok(())
    }

    /// Delete a key from the trie
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if the key is found and deleted
    /// - `Ok(false)` if the key is not found
    /// - `Err(e)` if other error occurs
    #[instrument(level = "trace", skip_all)]
    #[inline]
    pub fn delete<Db: KVDatabase, KEY: AsRef<[u8]>>(
        &mut self,
        db: &NodeDb<Db>,
        key: KEY,
    ) -> Result<bool, H, Db> {
        let key = key.as_ref();
        trace!(key = hex::encode(key));
        let node_key = self.key_hasher.hash(key)?;
        trace!(node_key = ?node_key);
        self.delete_by_node_key(db, node_key)
    }

    /// Delete a key from the trie by node key
    ///
    /// # See also
    ///
    /// [`delete`](ZkTrie::delete)
    pub fn delete_by_node_key<Db: KVDatabase>(
        &mut self,
        db: &NodeDb<Db>,
        node_key: ZkHash,
    ) -> Result<bool, H, Db> {
        match self.delete_node(db, self.root.clone(), node_key, 0) {
            Ok((new_root, _)) => {
                self.root = new_root;
                Ok(true)
            }
            Err(ZkTrieError::NodeNotFound) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Commit changes of the trie to the database
    pub fn commit<Db: KVDatabase>(&mut self, db: &mut NodeDb<Db>) -> Result<(), H, Db> {
        if !self.is_dirty() {
            return Ok(());
        }

        // resolve all unresolved branch nodes
        self.root = LazyNodeHash::Hash(self.resolve_commit(db, self.root.clone())?);

        // clear dirty nodes
        self.dirty_branch_nodes.clear();
        self.dirty_leafs.clear();
        self.gc_nodes.retain(|node_hash| node_hash.is_resolved());

        Ok(())
    }

    /// Prove constructs a merkle proof for key.
    /// The result contains all encoded nodes on the path to the value at key.
    /// The value itself is also included in the last node and can be retrieved by verifying the proof.
    ///
    /// If the trie does not contain a value for key, the returned proof contains all
    /// nodes of the longest existing prefix of the key (at least the root node), ending
    /// with the node that proves the absence of the key.
    ///
    /// If the trie contain a non-empty leaf for key, the returned proof contains all
    /// nodes on the path to the leaf node, ending with the leaf node.
    #[instrument(level = "trace", skip_all)]
    pub fn prove<Db: KVDatabase, KEY: AsRef<[u8]>>(
        &self,
        db: &NodeDb<Db>,
        key: KEY,
    ) -> Result<Vec<Vec<u8>>, H, Db> {
        let key = key.as_ref();
        trace!(key = hex::encode(key));
        let node_key = self.key_hasher.hash(key)?;
        trace!(node_key = ?node_key);

        let mut next_hash = self.root.clone();
        let mut proof = Vec::with_capacity(H::TRIE_MAX_LEVELS + 1);
        for i in 0..H::TRIE_MAX_LEVELS {
            let n = self.get_node_by_hash(db, next_hash)?;
            proof.push(n.canonical_value(true));
            match n.node_type() {
                NodeType::Empty | NodeType::Leaf => break,
                _ => {
                    let (_, child_left, child_right) = n.as_branch().unwrap().as_parts();
                    next_hash = if get_path(&node_key, i) {
                        child_right.clone()
                    } else {
                        child_left.clone()
                    };
                }
            }
        }
        proof.push(MAGIC_NODE_BYTES.to_vec());
        Ok(proof)
    }

    /// Garbage collect the trie
    pub fn gc<Db: KVDatabase>(&mut self, db: &mut NodeDb<Db>) -> Result<(), H, Db> {
        if !db.gc_enabled() {
            warn!("garbage collection is disabled");
            return Ok(());
        }
        let is_dirty = self.is_dirty();
        let mut removed = 0;
        self.gc_nodes
            .retain(|node_hash| match node_hash.try_as_hash() {
                Some(node_hash) => match db.remove_node(node_hash) {
                    Ok(_) => {
                        removed += 1;
                        false
                    }
                    Err(e) => {
                        warn!("Failed to remove node from db: {}", e);
                        true
                    }
                },
                None => {
                    if is_dirty {
                        warn!("Unresolved hash found in gc_nodes, commit before run gc");
                        true
                    } else {
                        false
                    }
                }
            });
        trace!("garbage collection done, removed {removed} nodes");
        Ok(())
    }

    /// Run full garbage collection
    ///
    /// If a temporary purge store is provided,
    /// the trie will be traversed and all node hashes will be set to the temporary store.
    /// Otherwise, the trie will be traversed and all nodes will be collected into memory.
    ///
    /// # Notes
    ///
    /// This method will enable the gc support regardless of the current state.
    ///
    /// This method will traverse the trie and collect all nodes,
    /// then remove all nodes that are not in the trie.
    pub fn full_gc<Db: KVDatabase, T: KVDatabase>(
        &mut self,
        db: &mut NodeDb<Db>,
        mut tmp_purge_store: T,
    ) -> Result<(), H, Db> {
        if !db.is_gc_supported() {
            warn!("backend database does not support garbage collection, skipping");
            return Ok(());
        }
        if self.is_dirty() {
            warn!("dirty nodes found, commit before run full_gc");
            return Ok(());
        }
        let gc_enabled = db.gc_enabled();
        db.set_gc_enabled(true);

        // traverse the trie and collect all nodes
        for node in self.iter(db) {
            let node = node?;
            let node_hash = *node
                .get_or_calculate_node_hash()
                .map_err(ZkTrieError::Hash)?;
            tmp_purge_store
                .put(node_hash.as_slice(), &[])
                .map_err(|e| ZkTrieError::Other(Box::new(e)))?;
        }

        db.retain(|k| match tmp_purge_store.get(k) {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(e) => {
                error!("Failed to check node in purge store: {}", e);
                true
            }
        })
        .map_err(ZkTrieError::Db)?;
        db.set_gc_enabled(gc_enabled);

        Ok(())
    }

    /// Get an iterator of the trie
    pub fn iter<'a, Db: KVDatabase>(&'a self, db: &'a NodeDb<Db>) -> ZkTrieIterator<'a, H, Db, K> {
        ZkTrieIterator {
            trie: self,
            db,
            stack: vec![self.root.clone()],
        }
    }

    /// Get a node from the trie by node hash
    #[instrument(level = "trace", skip(self, db, node_hash))]
    pub fn get_node_by_hash<Db: KVDatabase>(
        &self,
        db: &NodeDb<Db>,
        node_hash: impl Into<LazyNodeHash>,
    ) -> Result<INode<H>, H, Db> {
        let node_hash = node_hash.into();
        if node_hash.is_zero().unwrap_or(false) {
            return Ok(INode::Owned(Node::<H>::empty()));
        }
        trace!(node_hash = ?node_hash);
        match node_hash {
            LazyNodeHash::Hash(node_hash) => {
                if let Some(node) = self.dirty_leafs.get(&node_hash) {
                    trace!("Found node in dirty leafs");
                    Ok(INode::Owned(node.clone()))
                } else {
                    let node_view = db
                        .get_node::<H>(&node_hash)
                        .map_err(ZkTrieError::Db)?
                        .ok_or(ZkTrieError::NodeNotFound)?;
                    Ok(INode::Archived(node_view))
                }
            }
            LazyNodeHash::LazyBranch(LazyBranchHash { index, .. }) => self
                .dirty_branch_nodes
                .get(index)
                .cloned()
                .map(INode::Owned)
                .ok_or(ZkTrieError::NodeNotFound),
        }
    }

    /// Get a node from the trie by node key
    #[instrument(level = "trace", skip(self, db, node_key))]
    pub fn get_node_by_key<Db: KVDatabase>(
        &self,
        db: &NodeDb<Db>,
        node_key: &ZkHash,
    ) -> Result<INode<H>, H, Db> {
        let mut next_hash = self.root.clone();
        for i in 0..H::TRIE_MAX_LEVELS {
            let n = self.get_node_by_hash(db, next_hash)?;
            match n.node_type() {
                NodeType::Empty => return Ok(INode::Owned(Node::<H>::empty())),
                NodeType::Leaf => {
                    let leaf = n.as_leaf().unwrap();
                    return if leaf.node_key() == *node_key {
                        Ok(n)
                    } else if i != H::TRIE_MAX_LEVELS - 1 {
                        // the node is compressed, we just reached another leaf node
                        Ok(INode::Owned(Node::<H>::empty()))
                    } else {
                        Err(ZkTrieError::NodeNotFound)
                    };
                }
                _ => {
                    let branch = n.as_branch().unwrap();
                    if get_path(node_key, i) {
                        next_hash = branch.child_right().clone();
                    } else {
                        next_hash = branch.child_left().clone();
                    }
                }
            }
        }
        Err(ZkTrieError::NodeNotFound)
    }

    /// Recursively adds a new leaf in the MT while updating the path
    ///
    /// # Returns
    /// The new added node hash, and a boolean indicating if added node is terminal
    #[instrument(level = "trace", skip_all, ret)]
    fn add_leaf<Db: KVDatabase>(
        &mut self,
        db: &NodeDb<Db>,
        leaf: Node<H>,
        curr_node_hash: LazyNodeHash,
        level: usize,
    ) -> Result<(LazyNodeHash, bool), H, Db> {
        if level >= H::TRIE_MAX_LEVELS {
            return Err(ZkTrieError::MaxLevelReached);
        }
        let n = self.get_node_by_hash(db, curr_node_hash.clone())?;
        match n.node_type() {
            NodeType::Empty => {
                let node_hash = *leaf
                    .get_or_calculate_node_hash()
                    .map_err(ZkTrieError::Hash)?;
                self.dirty_leafs.insert(node_hash, leaf);

                Ok((LazyNodeHash::Hash(node_hash), true))
            }
            NodeType::Leaf => {
                let curr_node_hash = *curr_node_hash.unwrap_ref();
                let new_leaf_node_hash = *leaf
                    .get_or_calculate_node_hash()
                    .map_err(ZkTrieError::Hash)?;

                let new_leaf_node_key = *leaf.as_leaf().unwrap().node_key();
                let current_leaf_node_key = *n.as_leaf().unwrap().node_key();
                if curr_node_hash == new_leaf_node_hash {
                    // leaf already stored
                    Ok((LazyNodeHash::Hash(new_leaf_node_hash), true))
                } else if new_leaf_node_key == current_leaf_node_key {
                    self.dirty_leafs.insert(new_leaf_node_hash, leaf);
                    self.gc_nodes.insert(curr_node_hash.into());
                    Ok((LazyNodeHash::Hash(new_leaf_node_hash), true))
                } else {
                    Ok((self.push_leaf(db, n, leaf, level)?, false))
                }
            }
            // branch node
            _ => {
                let (current_node_type, current_node_left_child, current_node_right_child) =
                    n.as_branch().unwrap().as_parts();
                let leaf_node_key = leaf.as_leaf().unwrap().node_key();

                let new_parent_node = if get_path(&leaf_node_key, level) {
                    // go right
                    let (new_node_hash, is_terminal) =
                        self.add_leaf(db, leaf, current_node_right_child.clone(), level + 1)?;
                    let new_node_type = if !is_terminal {
                        match current_node_type {
                            NodeType::BranchLTRT => NodeType::BranchLTRB,
                            NodeType::BranchLTRB => NodeType::BranchLTRB,
                            NodeType::BranchLBRT => NodeType::BranchLBRB,
                            NodeType::BranchLBRB => NodeType::BranchLBRB,
                            _ => unreachable!(),
                        }
                    } else {
                        current_node_type
                    };
                    Node::new_branch(
                        new_node_type,
                        current_node_left_child.clone(),
                        new_node_hash,
                    )
                } else {
                    // go left
                    let (new_node_hash, is_terminal) =
                        self.add_leaf(db, leaf, current_node_left_child.clone(), level + 1)?;
                    let new_node_type = if !is_terminal {
                        match current_node_type {
                            NodeType::BranchLTRT => NodeType::BranchLBRT,
                            NodeType::BranchLTRB => NodeType::BranchLBRB,
                            NodeType::BranchLBRT => NodeType::BranchLBRT,
                            NodeType::BranchLBRB => NodeType::BranchLBRB,
                            _ => unreachable!(),
                        }
                    } else {
                        current_node_type
                    };
                    Node::new_branch(
                        new_node_type,
                        new_node_hash,
                        current_node_right_child.clone(),
                    )
                };

                let lazy_hash = LazyNodeHash::LazyBranch(LazyBranchHash {
                    index: self.dirty_branch_nodes.len(),
                    resolved: new_parent_node.node_hash.clone(),
                });

                self.gc_nodes.insert(curr_node_hash);
                self.dirty_branch_nodes.push(new_parent_node);
                Ok((lazy_hash, false))
            }
        }
    }

    /// Recursively pushes an existing old leaf down until its path diverges
    /// from new leaf, at which point both leafs are stored, all while updating the
    /// path.
    ///
    /// # Returns
    /// The node of the parent of the old leaf and new leaf
    fn push_leaf<Db: KVDatabase>(
        &mut self,
        db: &NodeDb<Db>,
        old_leaf: INode<H>,
        new_leaf: Node<H>,
        level: usize,
    ) -> Result<LazyNodeHash, H, Db> {
        if level >= H::TRIE_MAX_LEVELS - 1 {
            return Err(ZkTrieError::MaxLevelReached);
        }

        let old_leaf_node_key = old_leaf.as_leaf().unwrap().node_key();
        let new_leaf_node_key = new_leaf.as_leaf().unwrap().node_key();

        let old_leaf_path = get_path(&old_leaf_node_key, level);
        let new_leaf_path = get_path(&new_leaf_node_key, level);

        let new_parent = if old_leaf_path == new_leaf_path {
            // Need to go deeper
            let next_parent = self.push_leaf(db, old_leaf, new_leaf, level + 1)?;
            if old_leaf_path {
                // both leaves are on the right
                // So, left child is empty, right child is a branch node
                Node::new_branch(NodeType::BranchLTRB, ZkHash::ZERO, next_parent)
            } else {
                // both leaves are on the left
                // So, left child is a branch node, right child is empty
                Node::new_branch(NodeType::BranchLBRT, next_parent, ZkHash::ZERO)
            }
        } else {
            // Diverged, store new leaf
            let old_leaf_hash = *old_leaf
                .get_or_calculate_node_hash()
                .map_err(ZkTrieError::Hash)?;
            let new_leaf_hash = *new_leaf
                .get_or_calculate_node_hash()
                .map_err(ZkTrieError::Hash)?;
            self.dirty_leafs.insert(new_leaf_hash, new_leaf);
            // create parent node
            if new_leaf_path {
                // new leaf is on the right
                Node::new_branch(NodeType::BranchLTRT, old_leaf_hash, new_leaf_hash)
            } else {
                // new leaf is on the left
                Node::new_branch(NodeType::BranchLTRT, new_leaf_hash, old_leaf_hash)
            }
        };

        let lazy_hash = LazyNodeHash::LazyBranch(LazyBranchHash {
            index: self.dirty_branch_nodes.len(),
            resolved: new_parent.node_hash.clone(),
        });

        self.dirty_branch_nodes.push(new_parent);
        Ok(lazy_hash)
    }

    fn delete_node<Db: KVDatabase>(
        &mut self,
        db: &NodeDb<Db>,
        root_hash: LazyNodeHash,
        node_key: ZkHash,
        level: usize,
    ) -> Result<(LazyNodeHash, bool), H, Db> {
        if level >= H::TRIE_MAX_LEVELS {
            return Err(ZkTrieError::MaxLevelReached);
        }
        let root = self.get_node_by_hash(db, root_hash.clone())?;
        match root.node_type() {
            NodeType::Empty => Err(ZkTrieError::NodeNotFound),
            NodeType::Leaf => {
                if root.as_leaf().unwrap().node_key() != node_key {
                    Err(ZkTrieError::NodeNotFound)
                } else {
                    self.gc_nodes.insert(root_hash);
                    Ok((LazyNodeHash::Hash(ZkHash::ZERO), true))
                }
            }
            _ => {
                let path = get_path(&node_key, level);
                let (node_type, child_left, child_right) = root.as_branch().unwrap().as_parts();
                let (child_hash, sibling_hash) = if path {
                    (child_right.clone(), child_left.clone())
                } else {
                    (child_left.clone(), child_right.clone())
                };

                let is_sibling_terminal = matches!(
                    (path, node_type),
                    (_, NodeType::BranchLTRT)
                        | (true, NodeType::BranchLTRB)
                        | (false, NodeType::BranchLBRT)
                );

                let (new_child_hash, is_new_child_terminal) =
                    self.delete_node(db, child_hash, node_key, level + 1)?;

                let (left_child, right_child, is_left_terminal, is_right_terminal) = if path {
                    (
                        sibling_hash,
                        new_child_hash,
                        is_sibling_terminal,
                        is_new_child_terminal,
                    )
                } else {
                    (
                        new_child_hash,
                        sibling_hash,
                        is_new_child_terminal,
                        is_sibling_terminal,
                    )
                };
                let new_node_type = if is_left_terminal && is_right_terminal {
                    let left_is_empty = left_child.unwrap_ref().is_zero();
                    let right_is_empty = right_child.unwrap_ref().is_zero();

                    // If both children are terminal and one of them is empty, prune the root node
                    // and return the non-empty child
                    if left_is_empty || right_is_empty {
                        if left_is_empty {
                            return Ok((right_child, true));
                        }
                        return Ok((left_child, true));
                    } else {
                        NodeType::BranchLTRT
                    }
                } else if is_left_terminal {
                    NodeType::BranchLTRB
                } else if is_right_terminal {
                    NodeType::BranchLBRT
                } else {
                    NodeType::BranchLBRB
                };

                let new_parent = Node::new_branch(new_node_type, left_child, right_child);

                let lazy_hash = LazyNodeHash::LazyBranch(LazyBranchHash {
                    index: self.dirty_branch_nodes.len(),
                    resolved: new_parent.node_hash.clone(),
                });

                self.gc_nodes.insert(root_hash);
                self.dirty_branch_nodes.push(new_parent);

                Ok((lazy_hash, false))
            }
        }
    }

    #[instrument(level = "trace", skip(self, db), ret)]
    fn resolve_commit<Db: KVDatabase>(
        &mut self,
        db: &mut NodeDb<Db>,
        node_hash: LazyNodeHash,
    ) -> Result<ZkHash, H, Db> {
        match node_hash {
            LazyNodeHash::Hash(node_hash) => {
                if let Some(node) = self.dirty_leafs.remove(&node_hash) {
                    db.put_node(node).map_err(ZkTrieError::Db)?;
                }
                Ok(node_hash)
            }
            _ => match self.get_node_by_hash(db, node_hash)? {
                INode::Owned(node) => {
                    let branch = node.as_branch().unwrap();
                    self.resolve_commit(db, branch.child_left().clone())?;
                    self.resolve_commit(db, branch.child_right().clone())?;
                    let node_hash = *node
                        .get_or_calculate_node_hash()
                        .map_err(ZkTrieError::Hash)?;
                    db.put_node(node).map_err(ZkTrieError::Db)?;
                    Ok(node_hash)
                }
                INode::Archived(viewer) => Ok(viewer.node_hash),
            },
        }
    }
}

impl<'a, H: HashScheme, Db: KVDatabase, K: KeyHasher<H>> Debug for ZkTrieIterator<'a, H, Db, K> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ZkTrieIterator")
            .field("trie", &self.trie)
            .finish()
    }
}

impl<'a, H: HashScheme, Db: KVDatabase, K: KeyHasher<H>> Iterator for ZkTrieIterator<'a, H, Db, K> {
    type Item = Result<INode<H>, H, Db>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node_hash) = self.stack.pop() {
            return match self.trie.get_node_by_hash(self.db, node_hash) {
                Ok(node) => {
                    if node.is_branch() {
                        let branch = node.as_branch().expect("infalible");
                        self.stack.push(branch.child_left().clone());
                        self.stack.push(branch.child_right().clone());
                    }
                    Some(Ok(node))
                }
                Err(e) => Some(Err(e)),
            };
        }
        None
    }
}

#[inline(always)]
fn get_path(node_key: &ZkHash, level: usize) -> bool {
    node_key.as_slice()[HASH_SIZE - level / 8 - 1] & (1 << (level % 8)) != 0
}
