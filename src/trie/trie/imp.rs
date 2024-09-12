use super::*;
use std::fmt::{Debug, Formatter};

type Result<T, H, DB, CacheDb> = std::result::Result<
    T,
    ZkTrieError<
        <H as HashScheme>::Error,
        <DB as KVDatabase>::Error,
        <CacheDb as KVDatabase>::Error,
    >,
>;

impl<const MAX_LEVEL: usize, H: HashScheme> Default for ZkTrie<MAX_LEVEL, H> {
    fn default() -> Self {
        Self::new(HashMapDb::new(), HashMapDb::new())
    }
}

impl<const MAX_LEVEL: usize, H: HashScheme, Db: KVDatabase, CacheDb: KVDatabase> Debug
    for ZkTrie<MAX_LEVEL, H, Db, CacheDb>
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
impl<const MAX_LEVEL: usize, H: HashScheme, Db: KVDatabase, CacheDb: KVDatabase>
    ZkTrie<MAX_LEVEL, H, Db, CacheDb>
{
    /// Create a new zkTrie
    #[inline(always)]
    pub fn new(db: Db, cache_db: CacheDb) -> Self {
        Self::new_with_root(db, cache_db, ZkHash::default()).expect("infallible")
    }

    /// Create a new zkTrie with a given root hash
    #[inline]
    pub fn new_with_root(db: Db, cache_db: CacheDb, root: ZkHash) -> Result<Self, H, Db, CacheDb> {
        let this = Self {
            db,
            key_cache: KeyCacheDb::new(cache_db),
            root: root.into(),
            dirty_branch_nodes: Vec::new(),
            dirty_leafs: HashMap::new(),
            _hash_scheme: std::marker::PhantomData,
        };

        this.get_node(&root)?;

        Ok(this)
    }

    /// Get a node from the trie
    pub fn get_node(&self, node_hash: impl Into<LazyNodeHash>) -> Result<Node<H>, H, Db, CacheDb> {
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

    /// Update the trie with a new key-values pair
    pub fn update(
        &mut self,
        key: &[u8],
        value_preimages: Vec<[u8; 32]>,
        compression_flags: u32,
    ) -> Result<(), H, Db, CacheDb> {
        let node_key = self.key_cache.get_or_compute_if_absent(key)?;
        let new_leaf = Node::new_leaf(node_key, value_preimages, compression_flags, None)
            .map_err(ZkTrieError::Hash)?;
        self.root = self.add_leaf(new_leaf, self.root.clone(), 0)?.0;
        Ok(())
    }

    /// Commit changes of the trie to the database
    pub fn commit(&mut self) -> Result<(), H, Db, CacheDb> {
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

    /// Recursively adds a new leaf in the MT while updating the path
    ///
    /// # Returns
    /// The new added node hash, and a boolean indicating if added node is terminal
    fn add_leaf(
        &mut self,
        leaf: Node<H>,
        curr_node_hash: LazyNodeHash,
        level: usize,
    ) -> Result<(LazyNodeHash, bool), H, Db, CacheDb> {
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

                let lazy_hash = LazyNodeHash::LazyBranch(LazyBranch {
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
    ) -> Result<LazyNodeHash, H, Db, CacheDb> {
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

        let lazy_hash = LazyNodeHash::LazyBranch(LazyBranch {
            index: self.dirty_branch_nodes.len(),
            resolved: new_parent.node_hash.clone(),
        });

        self.dirty_branch_nodes.push(new_parent);
        Ok(lazy_hash)
    }

    fn resolve_commit(&mut self, node_hash: LazyNodeHash) -> Result<ZkHash, H, Db, CacheDb> {
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
