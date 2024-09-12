use super::*;

use crate::trie::{DecodeValueBytes, EncodeValueBytes, LazyBranchHash};
use std::fmt::{Debug, Formatter};

type Result<T, H, DB> =
    std::result::Result<T, ZkTrieError<<H as HashScheme>::Error, <DB as KVDatabase>::Error>>;

impl<const MAX_LEVEL: usize, H: HashScheme> Default for ZkTrie<MAX_LEVEL, H> {
    fn default() -> Self {
        Self::new(HashMapDb::new(), NoCacheHasher)
    }
}

impl<const MAX_LEVEL: usize, H: HashScheme, Db: KVDatabase, K: KeyHasher<H>> Debug
    for ZkTrie<MAX_LEVEL, H, Db, K>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ZkTrie")
            .field("MAX_LEVEL", &MAX_LEVEL)
            .field("hash_scheme", &std::any::type_name::<H>())
            .field("root", &self.root)
            .field("is_dirty", &self.is_dirty())
            .finish()
    }
}
impl<const MAX_LEVEL: usize, H: HashScheme, Db: KVDatabase, K: KeyHasher<H>>
    ZkTrie<MAX_LEVEL, H, Db, K>
{
    /// Create a new zkTrie
    #[inline(always)]
    pub fn new(db: Db, key_hasher: K) -> Self {
        Self::new_with_root(db, key_hasher, ZkHash::default()).expect("infallible")
    }

    /// Create a new zkTrie with a given root hash
    #[inline]
    pub fn new_with_root(db: Db, key_hasher: K, root: ZkHash) -> Result<Self, H, Db> {
        let this = Self {
            db,
            key_hasher,
            root: root.into(),
            dirty_branch_nodes: Vec::new(),
            dirty_leafs: HashMap::new(),
            _hash_scheme: std::marker::PhantomData,
        };

        this.get_node(&root)?;

        Ok(this)
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
    pub fn get<const LEN: usize, T: DecodeValueBytes<LEN>>(&self, key: &[u8]) -> Result<T, H, Db> {
        let node_key = self.key_hasher.hash(key)?;
        let node = self.get_node(node_key)?;
        match node.node_type() {
            NodeType::Empty => Err(ZkTrieError::NodeNotFound),
            NodeType::Leaf => {
                let leaf = node.into_leaf().unwrap();
                let values = leaf.into_value_preimages();
                let values: &[[u8; 32]; LEN] =
                    values
                        .as_ref()
                        .try_into()
                        .map_err(|_| ZkTrieError::UnexpectValueLength {
                            expected: LEN,
                            actual: values.len(),
                        })?;

                Ok(T::decode_values_bytes(values))
            }
            _ => Err(ZkTrieError::ExpectLeafNode),
        }
    }

    /// Update the trie with a new key-value pair, which value can be encoded to bytes
    #[inline(always)]
    pub fn update<T: EncodeValueBytes>(&mut self, key: &[u8], value: T) -> Result<(), H, Db> {
        let (values, compression_flags) = value.encode_values_bytes();
        self.raw_update(key, values, compression_flags)
    }

    /// Update the trie with a new key-values pair
    pub fn raw_update(
        &mut self,
        key: &[u8],
        value_preimages: Vec<[u8; 32]>,
        compression_flags: u32,
    ) -> Result<(), H, Db> {
        let node_key = self.key_hasher.hash(key)?;
        let new_leaf = Node::new_leaf(node_key, value_preimages, compression_flags, None)
            .map_err(ZkTrieError::Hash)?;
        self.root = self.add_leaf(new_leaf, self.root.clone(), 0)?.0;
        Ok(())
    }

    /// Delete a key from the trie
    pub fn delete(&mut self, key: &[u8]) -> Result<(), H, Db> {
        let node_key = self.key_hasher.hash(key)?;
        self.root = self.delete_node(self.root.clone(), node_key, 0)?.0;
        Ok(())
    }

    /// Commit changes of the trie to the database
    pub fn commit(&mut self) -> Result<(), H, Db> {
        if !self.is_dirty() {
            return Ok(());
        }

        // resolve all unresolved branch nodes
        self.root = LazyNodeHash::Hash(self.resolve_commit(self.root.clone())?);

        // clear dirty nodes
        self.dirty_branch_nodes.clear();
        self.dirty_leafs.clear();

        Ok(())
    }

    /// Get a node from the trie
    pub fn get_node(&self, node_hash: impl Into<LazyNodeHash>) -> Result<Node<H>, H, Db> {
        let node_hash = node_hash.into();
        if node_hash.is_zero() {
            return Ok(Node::<H>::empty());
        }
        match node_hash {
            LazyNodeHash::Hash(node_hash) => {
                if let Some(node) = self.dirty_leafs.get(&node_hash) {
                    Ok(node.clone())
                } else {
                    let node = self
                        .db
                        .get(node_hash.as_ref())
                        .map_err(ZkTrieError::Db)?
                        .map(|bytes| Node::try_from(bytes.as_ref()))
                        .ok_or(ZkTrieError::NodeNotFound)??;
                    // # Safety
                    // We just retrieved the node from the database, so it should be valid
                    unsafe { node.set_node_hash(node_hash) }
                    Ok(node)
                }
            }
            LazyNodeHash::LazyBranch(LazyBranchHash { index, .. }) => self
                .dirty_branch_nodes
                .get(index)
                .cloned()
                .ok_or(ZkTrieError::NodeNotFound),
        }
    }

    /// Recursively adds a new leaf in the MT while updating the path
    ///
    /// # Returns
    /// The new added node hash, and a boolean indicating if added node is terminal
    fn add_leaf(
        &mut self,
        leaf: Node<H>,
        curr_node_hash: LazyNodeHash,
        level: usize,
    ) -> Result<(LazyNodeHash, bool), H, Db> {
        if level >= MAX_LEVEL {
            return Err(ZkTrieError::MaxLevelReached);
        }
        let n = self.get_node(curr_node_hash.clone())?;
        match n.node_type() {
            NodeType::Empty => {
                // # Safety
                // leaf node always has a node hash
                let node_hash = unsafe { *leaf.get_node_hash_unchecked() };
                self.dirty_leafs.insert(node_hash, leaf);

                Ok((LazyNodeHash::Hash(node_hash), true))
            }
            NodeType::Leaf => {
                let curr_node_hash = *curr_node_hash.unwrap_ref();
                // # Safety
                // leaf node always has a node hash
                let new_leaf_node_hash = unsafe { *leaf.get_node_hash_unchecked() };

                let new_leaf_node_key = leaf.as_leaf().unwrap().node_key();
                let current_leaf_node_key = n.as_leaf().unwrap().node_key();
                if curr_node_hash == new_leaf_node_hash {
                    // leaf already stored
                    Ok((LazyNodeHash::Hash(new_leaf_node_hash), true))
                } else if new_leaf_node_key == current_leaf_node_key {
                    self.dirty_leafs.insert(new_leaf_node_hash, leaf);
                    Ok((LazyNodeHash::Hash(new_leaf_node_hash), true))
                } else {
                    Ok((self.push_leaf(n, leaf, level)?, false))
                }
            }
            // branch node
            _ => {
                let (current_node_type, current_node_left_child, current_node_right_child) =
                    n.into_branch().unwrap().into_parts();
                let leaf_node_key = leaf.as_leaf().unwrap().node_key();

                let new_parent_node = if get_path(leaf_node_key, level) {
                    // go right
                    let (new_node_hash, is_terminal) =
                        self.add_leaf(leaf, current_node_right_child, level + 1)?;
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
                    Node::new_branch(new_node_type, current_node_left_child, new_node_hash)
                } else {
                    // go left
                    let (new_node_hash, is_terminal) =
                        self.add_leaf(leaf, current_node_left_child, level + 1)?;
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
                    Node::new_branch(new_node_type, new_node_hash, current_node_right_child)
                };

                let lazy_hash = LazyNodeHash::LazyBranch(LazyBranchHash {
                    index: self.dirty_branch_nodes.len(),
                    resolved: new_parent_node.node_hash.clone(),
                });

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
    fn push_leaf(
        &mut self,
        old_leaf: Node<H>,
        new_leaf: Node<H>,
        level: usize,
    ) -> Result<LazyNodeHash, H, Db> {
        if level >= MAX_LEVEL - 1 {
            return Err(ZkTrieError::MaxLevelReached);
        }

        let old_leaf_node_key = old_leaf.as_leaf().unwrap().node_key();
        let new_leaf_node_key = new_leaf.as_leaf().unwrap().node_key();

        let old_leaf_path = get_path(old_leaf_node_key, level);
        let new_leaf_path = get_path(new_leaf_node_key, level);

        let new_parent = if old_leaf_path == new_leaf_path {
            // Need to go deeper
            let next_parent = self.push_leaf(old_leaf, new_leaf, level + 1)?;
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
            // # Safety
            // leaf node always has a node hash
            let old_leaf_hash = unsafe { *old_leaf.get_node_hash_unchecked() };
            let new_leaf_hash = unsafe { *new_leaf.get_node_hash_unchecked() };
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

    fn delete_node(
        &mut self,
        root_hash: LazyNodeHash,
        node_key: ZkHash,
        level: usize,
    ) -> Result<(LazyNodeHash, bool), H, Db> {
        if level >= MAX_LEVEL {
            return Err(ZkTrieError::MaxLevelReached);
        }
        let root = self.get_node(root_hash)?;
        match root.node_type() {
            NodeType::Empty => Err(ZkTrieError::NodeNotFound),
            NodeType::Leaf => {
                if root.as_leaf().unwrap().node_key() != &node_key {
                    Err(ZkTrieError::NodeNotFound)
                } else {
                    Ok((LazyNodeHash::Hash(ZkHash::ZERO), true))
                }
            }
            _ => {
                let path = get_path(&node_key, level);
                let (node_type, child_left, child_right) = root.into_branch().unwrap().into_parts();
                let (child_hash, sibling_hash) = if path {
                    (child_right, child_left)
                } else {
                    (child_left, child_right)
                };

                let is_sibling_terminal = match (path, node_type) {
                    (_, NodeType::BranchLTRT) => true,
                    (true, NodeType::BranchLTRB) => true,
                    (false, NodeType::BranchLBRT) => true,
                    _ => false,
                };

                let (new_child_hash, is_new_child_terminal) =
                    self.delete_node(child_hash, node_key, level + 1)?;

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

                self.dirty_branch_nodes.push(new_parent);

                Ok((lazy_hash, false))
            }
        }
    }

    fn resolve_commit(&mut self, node_hash: LazyNodeHash) -> Result<ZkHash, H, Db> {
        match node_hash {
            LazyNodeHash::Hash(node_hash) => {
                if let Some(node) = self.dirty_leafs.remove(&node_hash) {
                    self.db
                        .put_owned(
                            node_hash.to_vec().into_boxed_slice(),
                            node.canonical_value().into_boxed_slice(),
                        )
                        .map_err(ZkTrieError::Db)?;
                }
                Ok(node_hash)
            }
            _ => {
                let node = self.get_node(node_hash)?;
                let branch = node.as_branch().unwrap();
                self.resolve_commit(branch.child_left().clone())?;
                self.resolve_commit(branch.child_right().clone())?;
                let node_hash = *node
                    .get_or_calculate_node_hash()
                    .map_err(ZkTrieError::Hash)?;
                self.db
                    .put_owned(
                        node_hash.to_vec().into_boxed_slice(),
                        node.canonical_value().into_boxed_slice(),
                    )
                    .map_err(ZkTrieError::Db)?;
                Ok(node_hash)
            }
        }
    }
}

#[inline(always)]
fn get_path(node_key: &ZkHash, level: usize) -> bool {
    node_key.as_slice()[HASH_SIZE - level / 8 - 1] & (1 << (level % 8)) != 0
}