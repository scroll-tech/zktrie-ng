#![allow(missing_debug_implementations)]
#![allow(clippy::unit_arg)]

use super::*;
use alloy_primitives::bytes::Bytes;
use rkyv::rancor;
use rkyv::util::AlignedVec;
use std::fmt::Debug;

/// An archived [`Node`].
#[derive(Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(archived = ArchivedNode, derive(Debug, Hash, PartialEq, Eq))]
pub struct NodeForArchive {
    node_hash: Option<ZkHash>,
    data: NodeKindForArchive,
}

impl<H> From<Node<H>> for NodeForArchive {
    fn from(node: Node<H>) -> Self {
        Self {
            node_hash: node.node_hash.get().copied(),
            data: if node.data.is_empty() {
                NodeKindForArchive::Empty
            } else if node.data.is_leaf() {
                NodeKindForArchive::Leaf(node.data.as_leaf().unwrap().clone().into())
            } else {
                NodeKindForArchive::Branch(node.data.as_branch().unwrap().clone().into())
            },
        }
    }
}

impl<H> Node<H> {
    /// Archive the node into bytes
    pub fn archived(self) -> AlignedVec {
        rkyv::to_bytes::<rancor::Error>(&NodeForArchive::from(self)).expect("infallible")
    }
}

/// Three kinds of nodes in the merkle tree.
#[derive(Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(archived = ArchivedNodeKind, derive(Debug, Hash, PartialEq, Eq))]
pub enum NodeKindForArchive {
    /// An empty node.
    Empty,
    /// A leaf node.
    Leaf(LeafNodeForArchive),
    /// A branch node.
    Branch(BranchNodeForArchive),
}

#[derive(Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(archived = ArchivedLeafNode, derive(Debug, Hash, PartialEq, Eq))]
pub struct LeafNodeForArchive {
    node_key: ZkHash,
    node_key_preimage: Option<[u8; 32]>,
    value_preimages: Vec<[u8; 32]>,
    compress_flags: u32,
    value_hash: Option<ZkHash>,
}

impl From<LeafNode> for LeafNodeForArchive {
    fn from(node: LeafNode) -> Self {
        Self {
            node_key: node.node_key,
            node_key_preimage: node.node_key_preimage,
            value_preimages: node.value_preimages,
            compress_flags: node.compress_flags,
            value_hash: node.value_hash.get().copied(),
        }
    }
}

#[derive(Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(archived = ArchivedBranchNode, derive(Debug, Hash, PartialEq, Eq))]
pub struct BranchNodeForArchive {
    node_type: u8,
    child_left: ZkHash,
    child_right: ZkHash,
}

impl From<BranchNode> for BranchNodeForArchive {
    fn from(node: BranchNode) -> Self {
        Self {
            node_type: node.node_type as u8,
            child_left: *node.child_left.unwrap_ref(),
            child_right: *node.child_right.unwrap_ref(),
        }
    }
}

impl ArchivedNodeKind {
    /// is empty node
    #[inline]
    pub fn is_empty(&self) -> bool {
        matches!(self, ArchivedNodeKind::Empty)
    }

    /// is leaf node
    #[inline]
    pub fn is_leaf(&self) -> bool {
        matches!(self, ArchivedNodeKind::Leaf(_))
    }

    /// is branch node
    #[inline]
    pub fn is_branch(&self) -> bool {
        matches!(self, ArchivedNodeKind::Branch(_))
    }

    /// as leaf node
    #[inline]
    pub fn as_leaf(&self) -> Option<&ArchivedLeafNode> {
        match self {
            ArchivedNodeKind::Leaf(leaf) => Some(leaf),
            _ => None,
        }
    }

    /// as branch node
    #[inline]
    pub fn as_branch(&self) -> Option<&ArchivedBranchNode> {
        match self {
            ArchivedNodeKind::Branch(branch) => Some(branch),
            _ => None,
        }
    }
}

/// A leaf node that may be archived.
#[derive(Clone)]
pub enum ILeafNode<'a> {
    /// Owned leaf node.
    Owned(&'a LeafNode),
    /// Archived leaf node.
    Archived(&'a ArchivedLeafNode),
}

/// A branch node that may be archived.
#[derive(Clone)]
pub enum IBranchNode<'a> {
    /// Owned branch node.
    Owned(&'a BranchNode),
    /// Archived branch node.
    Archived(&'a ArchivedBranchNode),
}

/// A viewer to a chunk of archived bytes that represents a node.
#[derive(Clone, Debug)]
pub struct NodeViewer {
    pub(crate) data: Bytes,
    pub(crate) node_hash: ZkHash,
}

/// A node that may be owned or archived.
#[derive(Clone)]
pub enum INode<H> {
    /// Owned node.
    Owned(Node<H>),
    /// Archived node.
    Archived(NodeViewer),
}

impl ArchivedLeafNode {
    /// Get the `node_key` stored in a leaf node.
    #[inline]
    pub fn node_key(&self) -> ZkHash {
        (&self.node_key).into()
    }

    /// Get the original key value that derives the `node_key`, kept here only for proof.
    #[inline]
    pub fn node_key_preimage(&self) -> Option<&[u8; 32]> {
        self.node_key_preimage.as_ref()
    }

    /// Get the value preimages stored in a leaf node.
    #[inline]
    pub fn value_preimages(&self) -> &[[u8; 32]] {
        &self.value_preimages
    }

    /// Get the compress flags stored in a leaf node.
    #[inline]
    pub fn compress_flags(&self) -> u32 {
        self.compress_flags.into()
    }

    /// Get the `value_hash` of the leaf node.
    #[inline]
    pub fn value_hash(&self) -> Option<ZkHash> {
        self.value_hash.as_ref().map(|hash| hash.into())
    }

    /// Get the `value_hash`
    #[inline]
    pub fn get_or_calc_value_hash<H: HashScheme>(&self) -> Result<ZkHash, H::Error> {
        match self.value_hash() {
            Some(hash) => Ok(hash),
            None => self.calc_value_hash::<H>(),
        }
    }

    /// Calculate the `value_hash`
    #[inline]
    pub fn calc_value_hash<H: HashScheme>(&self) -> Result<ZkHash, H::Error> {
        H::hash_bytes_array(self.value_preimages(), self.compress_flags())
    }
}

impl ILeafNode<'_> {
    /// Get the `node_key` stored in a leaf node.
    #[inline]
    pub fn node_key(&self) -> ZkHash {
        match self {
            ILeafNode::Owned(leaf) => leaf.node_key(),
            ILeafNode::Archived(leaf) => leaf.node_key(),
        }
    }

    /// Get the original key value that derives the `node_key`, kept here only for proof.
    #[inline]
    pub fn node_key_preimage(&self) -> Option<&[u8; 32]> {
        match self {
            ILeafNode::Owned(leaf) => leaf.node_key_preimage(),
            ILeafNode::Archived(leaf) => leaf.node_key_preimage(),
        }
    }

    /// Get the value preimages stored in a leaf node.
    #[inline]
    pub fn value_preimages(&self) -> &[[u8; 32]] {
        match self {
            ILeafNode::Owned(leaf) => leaf.value_preimages(),
            ILeafNode::Archived(leaf) => leaf.value_preimages(),
        }
    }

    /// Get the compress flags stored in a leaf node.
    #[inline]
    pub fn compress_flags(&self) -> u32 {
        match self {
            ILeafNode::Owned(leaf) => leaf.compress_flags(),
            ILeafNode::Archived(leaf) => leaf.compress_flags(),
        }
    }

    /// Get the `value_hash` of the leaf node.
    #[inline]
    pub fn value_hash(&self) -> Option<ZkHash> {
        match self {
            ILeafNode::Owned(leaf) => leaf.value_hash(),
            ILeafNode::Archived(leaf) => leaf.value_hash(),
        }
    }

    /// Get the `value_hash`
    #[inline]
    pub fn get_or_calc_value_hash<H: HashScheme>(&self) -> Result<ZkHash, H::Error> {
        match self {
            ILeafNode::Owned(leaf) => leaf.get_or_calc_value_hash::<H>(),
            ILeafNode::Archived(leaf) => leaf.get_or_calc_value_hash::<H>(),
        }
    }
}

impl<'a> IBranchNode<'a> {
    /// Get the node type.
    #[inline]
    pub fn node_type(&self) -> NodeType {
        match self {
            IBranchNode::Owned(branch) => branch.node_type,
            IBranchNode::Archived(branch) => branch.node_type(),
        }
    }

    /// Get the left child hash.
    #[inline]
    pub fn child_left(&self) -> LazyNodeHash {
        match self {
            IBranchNode::Owned(branch) => branch.child_left.clone(),
            IBranchNode::Archived(branch) => branch.child_left(),
        }
    }

    /// Get the right child hash.
    #[inline]
    pub fn child_right(&self) -> LazyNodeHash {
        match self {
            IBranchNode::Owned(branch) => branch.child_right.clone(),
            IBranchNode::Archived(branch) => branch.child_right(),
        }
    }

    /// As the parts
    #[inline]
    pub fn as_parts(&self) -> (NodeType, LazyNodeHash, LazyNodeHash) {
        (self.node_type(), self.child_left(), self.child_right())
    }
}

impl ArchivedBranchNode {
    /// Get the node type.
    #[inline]
    pub fn node_type(&self) -> NodeType {
        NodeType::from_u8(self.node_type).expect("invalid node type")
    }

    /// Get the left child hash.
    #[inline]
    pub fn child_left(&self) -> LazyNodeHash {
        LazyNodeHash::Hash((&self.child_left).into())
    }

    /// Get the right child hash.
    #[inline]
    pub fn child_right(&self) -> LazyNodeHash {
        LazyNodeHash::Hash((&self.child_right).into())
    }

    /// As the parts
    #[inline]
    pub fn as_parts(&self) -> (NodeType, LazyNodeHash, LazyNodeHash) {
        (self.node_type(), self.child_left(), self.child_right())
    }
}

impl ArchivedNode {
    /// Get the node type.
    #[inline]
    pub fn node_type(&self) -> NodeType {
        if self.data.is_empty() {
            return Empty;
        }
        if self.data.is_leaf() {
            return Leaf;
        }
        self.data.as_branch().expect("infallible").node_type()
    }

    /// check if the node is branch node
    #[inline]
    pub fn is_branch(&self) -> bool {
        self.data.as_branch().is_some()
    }

    /// check if the node is 'terminated', i.e. empty or leaf node
    #[inline]
    pub fn is_terminal(&self) -> bool {
        !self.is_branch()
    }

    /// Try as a leaf node.
    #[inline]
    pub fn as_leaf(&self) -> Option<&ArchivedLeafNode> {
        self.data.as_leaf()
    }

    /// Try as a branch node.
    #[inline]
    pub fn as_branch(&self) -> Option<&ArchivedBranchNode> {
        self.data.as_branch()
    }

    /// Encode the node into canonical bytes.
    pub fn canonical_value(&self, include_key_preimage: bool) -> Vec<u8> {
        match &self.data {
            ArchivedNodeKind::Leaf(leaf) => {
                let mut bytes = Vec::with_capacity(
                    1 + HASH_SIZE
                        + core::mem::size_of::<u32>()
                        + 32 * leaf.value_preimages.len()
                        + 1,
                );
                bytes.push(Leaf as u8);
                bytes.extend_from_slice(leaf.node_key.0.as_ref());
                let mark = (leaf.compress_flags << 8) + leaf.value_preimages.len() as u32;
                bytes.extend_from_slice(&mark.to_le_bytes());
                for preimage in leaf.value_preimages.iter() {
                    bytes.extend_from_slice(preimage);
                }
                if include_key_preimage && leaf.node_key_preimage.is_some() {
                    let preimage = leaf.node_key_preimage.as_ref().unwrap();
                    bytes.push(preimage.len() as u8);
                    bytes.extend_from_slice(preimage);
                } else {
                    // do not store node_key_preimage
                    bytes.push(0);
                }
                bytes
            }
            ArchivedNodeKind::Branch(branch) => {
                let mut bytes = Vec::with_capacity(1 + 2 * HASH_SIZE);
                bytes.push(branch.node_type);
                bytes.extend_from_slice(branch.child_left.0.as_ref());
                bytes.extend_from_slice(branch.child_right.0.as_ref());
                bytes
            }
            ArchivedNodeKind::Empty => {
                vec![Empty as u8]
            }
        }
    }

    /// Calculate the node hash.
    pub fn calculate_node_hash<H: HashScheme>(&self) -> Result<ZkHash, H::Error> {
        if self.data.is_empty() {
            return Ok(ZkHash::ZERO);
        }
        if let Some(leaf) = self.as_leaf() {
            let value_hash = leaf.calc_value_hash::<H>()?;
            return H::hash(Leaf as u64, [leaf.node_key(), value_hash]);
        }
        let branch = self.as_branch().unwrap();
        H::hash(
            branch.node_type() as u64,
            [(&branch.child_left).into(), (&branch.child_right).into()],
        )
    }
}

impl NodeViewer {
    /// View the archived node.
    pub fn view(&self) -> &ArchivedNode {
        // SAFETY: The bytes are guaranteed to be a valid archived node
        unsafe { rkyv::access_unchecked::<ArchivedNode>(self.data.as_ref()) }
    }
}

impl<H: HashScheme> INode<H> {
    /// Try get node hash.
    #[inline]
    pub fn node_hash(&self) -> Option<&ZkHash> {
        match self {
            INode::Owned(node) => node.node_hash.get(),
            INode::Archived(node) => Some(&node.node_hash),
        }
    }

    /// Get the node hash or calculate it if not exists.
    ///
    /// # Panics
    ///
    /// Panics if the node is owned and the lazy hash is not resolved.
    #[inline]
    pub fn get_or_calculate_node_hash(&self) -> Result<&ZkHash, H::Error> {
        match self {
            INode::Owned(node) => node.get_or_calculate_node_hash(),
            INode::Archived(node) => Ok(&node.node_hash),
        }
    }

    /// Get the node hash unchecked
    ///
    /// # Safety
    ///
    /// Caller must ensure that the hash is resolved.
    #[inline]
    pub unsafe fn get_node_hash_unchecked(&self) -> &ZkHash {
        match self {
            INode::Owned(node) => node.get_node_hash_unchecked(),
            INode::Archived(node) => &node.node_hash,
        }
    }

    /// Get the node type.
    #[inline]
    pub fn node_type(&self) -> NodeType {
        match self {
            INode::Owned(node) => node.node_type(),
            INode::Archived(node) => node.view().node_type(),
        }
    }

    /// check if the node is branch node
    #[inline]
    pub fn is_branch(&self) -> bool {
        match self {
            INode::Owned(node) => node.is_branch(),
            INode::Archived(node) => node.view().is_branch(),
        }
    }

    /// check if the node is 'terminated', i.e. empty or leaf node
    #[inline]
    pub fn is_terminal(&self) -> bool {
        match self {
            INode::Owned(node) => node.is_terminal(),
            INode::Archived(node) => node.view().is_terminal(),
        }
    }

    /// Try as a leaf node.
    #[inline]
    pub fn as_leaf(&self) -> Option<ILeafNode> {
        match self {
            INode::Owned(node) => node.as_leaf().map(ILeafNode::Owned),
            INode::Archived(node) => node.view().as_leaf().map(ILeafNode::Archived),
        }
    }

    /// Try as a branch node.
    #[inline]
    pub fn as_branch(&self) -> Option<IBranchNode> {
        match self {
            INode::Owned(node) => node.as_branch().map(IBranchNode::Owned),
            INode::Archived(node) => node.view().as_branch().map(IBranchNode::Archived),
        }
    }

    /// Encode the node into canonical bytes.
    ///
    /// # Panics
    ///
    /// Panics if it's owned and the lazy hash is not resolved.
    pub fn canonical_value(&self, include_key_preimage: bool) -> Vec<u8> {
        match self {
            INode::Owned(node) => node.canonical_value(include_key_preimage),
            INode::Archived(node) => node.view().canonical_value(include_key_preimage),
        }
    }
}
