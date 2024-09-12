use super::*;
use once_cell::sync::Lazy;

impl From<ZkHash> for LazyNodeHash {
    fn from(hash: ZkHash) -> Self {
        LazyNodeHash::Hash(hash)
    }
}

impl From<&ZkHash> for LazyNodeHash {
    fn from(hash: &ZkHash) -> Self {
        LazyNodeHash::Hash(*hash)
    }
}

impl LazyNodeHash {
    /// Check if the hash value is zero.
    ///
    /// If the hash is lazy and unresolved, `false` will be returned.
    pub fn is_zero(&self) -> bool {
        match self {
            LazyNodeHash::Hash(hash) => hash.is_zero(),
            LazyNodeHash::LazyBranch(LazyBranch { resolved, .. }) => {
                resolved.get().map_or(false, ZkHash::is_zero)
            }
        }
    }

    /// Unwrap the hash value
    ///
    /// # Panics
    ///
    /// Panics if the lazy hash is not resolved.
    pub fn unwrap_ref(&self) -> &ZkHash {
        match self {
            LazyNodeHash::Hash(hash) => hash,
            LazyNodeHash::LazyBranch(LazyBranch { resolved, .. }) => resolved.get().unwrap(),
        }
    }
}

impl LeafNode {
    /// Get the `node_key` stored in a leaf node.
    #[inline]
    pub fn node_key(&self) -> &ZkHash {
        &self.node_key
    }

    /// Get the original key value that derives the `node_key`, kept here only for proof.
    #[inline]
    pub fn node_key_preimage(&self) -> Option<[u8; 32]> {
        self.node_key_preimage
    }

    /// Get the value preimages stored in a leaf node.
    #[inline]
    pub fn value_preimages(&self) -> &[[u8; 32]] {
        &self.value_preimages
    }

    /// Get the compress flags stored in a leaf node.
    #[inline]
    pub fn compress_flags(&self) -> u32 {
        self.compress_flags
    }

    /// Get the `value_hash`
    #[inline]
    pub fn value_hash<H: HashScheme>(&self) -> &ZkHash {
        &self.value_hash
    }
}

impl BranchNode {
    /// Get the node type.
    #[inline]
    pub fn node_type(&self) -> NodeType {
        self.node_type
    }

    /// Get the left child hash.
    #[inline]
    pub fn child_left(&self) -> &LazyNodeHash {
        &self.child_left
    }

    /// Get the right child hash.
    #[inline]
    pub fn child_right(&self) -> &LazyNodeHash {
        &self.child_right
    }

    /// Into the parts
    #[inline]
    pub fn into_parts(self) -> (NodeType, LazyNodeHash, LazyNodeHash) {
        (self.node_type, self.child_left, self.child_right)
    }
}

impl<H: HashScheme> Node<H> {
    /// Empty node.
    pub fn empty() -> Node<H> {
        static EMPTY_HASH: Lazy<Arc<OnceCell<ZkHash>>> =
            Lazy::new(|| Arc::new(OnceCell::with_value(ZkHash::ZERO)));
        Node {
            node_hash: EMPTY_HASH.clone(),
            data: NodeKind::Empty,
            _hash_scheme: std::marker::PhantomData,
        }
    }

    /// Create a new branch node.
    pub fn new_branch(
        node_type: NodeType,
        child_left: impl Into<LazyNodeHash>,
        child_right: impl Into<LazyNodeHash>,
    ) -> Self {
        Node {
            node_hash: Arc::new(OnceCell::new()),
            data: NodeKind::Branch(BranchNode {
                node_type,
                child_left: child_left.into(),
                child_right: child_right.into(),
            }),
            _hash_scheme: std::marker::PhantomData,
        }
    }

    /// Create a new leaf node.
    pub fn new_leaf(
        node_key: ZkHash,
        value_preimages: Vec<[u8; 32]>,
        compress_flags: u32,
        node_key_preimage: Option<[u8; 32]>,
    ) -> Result<Self, H::Error> {
        let value_hash = H::hash_bytes_array(&value_preimages, compress_flags)?;
        let node_hash = H::hash(Leaf as u64, [node_key, value_hash])?;
        Ok(Node {
            node_hash: Arc::new(OnceCell::with_value(node_hash)),
            data: NodeKind::Leaf(LeafNode {
                node_key,
                node_key_preimage,
                value_preimages: Arc::from(value_preimages.into_boxed_slice()),
                compress_flags,
                value_hash,
            }),
            _hash_scheme: std::marker::PhantomData,
        })
    }
}

impl<H: HashScheme> Node<H> {
    /// Get the node hash or calculate it if not exists.
    ///
    /// # Panics
    ///
    /// Panics if the lazy hash is not resolved.
    #[inline]
    pub fn get_or_calculate_node_hash(&self) -> Result<&ZkHash, H::Error> {
        match self.data {
            NodeKind::Empty | NodeKind::Leaf(_) => Ok(unsafe { self.node_hash.get_unchecked() }),
            NodeKind::Branch(ref branch) => {
                let left = branch.child_left.unwrap_ref();
                let right = branch.child_right.unwrap_ref();
                Ok(self
                    .node_hash
                    .get_or_try_init(|| H::hash(branch.node_type as u64, [*left, *right]))?)
            }
        }
    }

    /// Get the node hash unchecked
    ///
    /// # Safety
    ///
    /// Caller must ensure that the hash is resolved.
    #[inline]
    pub unsafe fn get_node_hash_unchecked(&self) -> &ZkHash {
        self.node_hash.get_unchecked()
    }

    /// Set the node hash.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it can break the internal consistency of the node.
    ///
    /// The caller must ensure that the hash is correct and consistent with the node data.
    #[inline]
    pub unsafe fn set_node_hash(&self, hash: ZkHash) {
        self.node_hash.set(hash).ok();
    }

    /// Get the node type.
    #[inline]
    pub fn node_type(&self) -> NodeType {
        match &self.data {
            NodeKind::Empty => Empty,
            NodeKind::Leaf(_) => Leaf,
            NodeKind::Branch(b) => b.node_type,
        }
    }

    /// check if the node is branch node
    #[inline]
    pub fn is_branch(&self) -> bool {
        matches!(self.data, NodeKind::Branch(_))
    }

    /// check if the node is 'terminated', i.e. empty or leaf node
    #[inline]
    pub fn is_terminal(&self) -> bool {
        self.node_type().is_terminal()
    }

    /// Try as a leaf node.
    #[inline]
    pub fn as_leaf(&self) -> Option<&LeafNode> {
        match &self.data {
            NodeKind::Leaf(leaf) => Some(leaf),
            _ => None,
        }
    }

    /// Try as a branch node.
    #[inline]
    pub fn as_branch(&self) -> Option<&BranchNode> {
        match &self.data {
            NodeKind::Branch(branch) => Some(branch),
            _ => None,
        }
    }

    /// Try into a leaf node.
    #[inline]
    pub fn into_leaf(self) -> Option<LeafNode> {
        match self.data {
            NodeKind::Leaf(leaf) => Some(leaf),
            _ => None,
        }
    }

    /// Try into a branch node.
    #[inline]
    pub fn into_branch(self) -> Option<BranchNode> {
        match self.data {
            NodeKind::Branch(branch) => Some(branch),
            _ => None,
        }
    }

    /// Encode the node into canonical bytes.
    ///
    /// # Panics
    ///
    /// Panics if the lazy hash is not resolved.
    pub fn canonical_value(&self) -> Vec<u8> {
        match &self.data {
            NodeKind::Leaf(leaf) => {
                let mut bytes = Vec::with_capacity(
                    1 + HASH_SIZE + size_of::<u32>() + 32 * leaf.value_preimages.len() + 1,
                );
                bytes.push(Leaf as u8);
                bytes.extend_from_slice(leaf.node_key.as_ref());
                let mark = (leaf.compress_flags << 8) + leaf.value_preimages.len() as u32;
                bytes.extend_from_slice(&mark.to_le_bytes());
                for preimage in leaf.value_preimages.iter() {
                    bytes.extend_from_slice(preimage);
                }
                bytes.push(0); // do not store node_key_preimage
                bytes
            }
            NodeKind::Branch(branch) => {
                let mut bytes = Vec::with_capacity(1 + 2 * HASH_SIZE);
                bytes.push(branch.node_type as u8);
                bytes.extend_from_slice(branch.child_left.unwrap_ref().as_ref());
                bytes.extend_from_slice(branch.child_right.unwrap_ref().as_ref());
                bytes
            }
            NodeKind::Empty => {
                vec![Empty as u8]
            }
        }
    }
}

impl<H: HashScheme> TryFrom<&[u8]> for Node<H> {
    type Error = ParseNodeError<H::Error>;

    fn try_from(mut bytes: &[u8]) -> Result<Self, Self::Error> {
        use ParseNodeError::*;

        let raw_node_type = read_u8(&mut bytes)?;
        let node_type =
            NodeType::from_u8(raw_node_type).ok_or_else(|| InvalidNodeType(raw_node_type))?;

        match node_type {
            BranchLTRT | BranchLTRB | BranchLBRT | BranchLBRB => {
                let child_left = read_hash::<H>(&mut bytes)?;
                let child_right = read_hash::<H>(&mut bytes)?;
                Ok(Self::new_branch(node_type, child_left, child_right))
            }
            Leaf => {
                let node_key = read_hash::<H>(&mut bytes)?;

                let mark = read_u32_le(&mut bytes)?;
                let preimage_len = (mark & 255) as usize;
                let compress_flags = mark >> 8;

                let mut value_preimages = Vec::with_capacity(preimage_len);
                for _ in 0..preimage_len {
                    value_preimages.push(read_bytes::<32, H::Error>(&mut bytes)?);
                }

                let key_preimage_size = read_u8(&mut bytes)? as usize;
                let node_key_preimage = if key_preimage_size > 0 {
                    Some(read_bytes::<32, H::Error>(&mut bytes)?)
                } else {
                    None
                };

                Ok(
                    Self::new_leaf(node_key, value_preimages, compress_flags, node_key_preimage)
                        .map_err(HashError)?,
                )
            }
            Empty => Ok(Self::empty()),
        }
    }
}

/// helper function to read u8 from bytes
#[inline]
fn read_u8<E>(bytes: &mut &[u8]) -> Result<u8, ParseNodeError<E>> {
    if bytes.is_empty() {
        return Err(ParseNodeError::Eof(0, 1));
    }
    let read = bytes[0];
    *bytes = &bytes[1..];
    Ok(read)
}

/// helper function to read u32 from bytes
#[inline]
fn read_u32_le<E>(bytes: &mut &[u8]) -> Result<u32, ParseNodeError<E>> {
    if bytes.len() < 4 {
        return Err(ParseNodeError::Eof(bytes.len(), 4));
    }
    let read = u32::from_le_bytes(bytes[..4].try_into().unwrap());
    *bytes = &bytes[4..];
    Ok(read)
}

/// helper function to read N bytes from bytes
#[inline]
fn read_bytes<const N: usize, E>(bytes: &mut &[u8]) -> Result<[u8; N], ParseNodeError<E>> {
    if bytes.len() < N {
        return Err(ParseNodeError::Eof(bytes.len(), N));
    }
    let read = bytes[..N].try_into().unwrap();
    *bytes = &bytes[N..];
    Ok(read)
}

/// helper function to read hash from bytes
#[inline]
fn read_hash<H: HashScheme>(bytes: &mut &[u8]) -> Result<ZkHash, ParseNodeError<H::Error>> {
    let read = H::new_hash_try_from_bytes(read_bytes::<HASH_SIZE, H::Error>(bytes)?.as_ref())
        .map_err(ParseNodeError::HashError)?;
    Ok(read)
}
