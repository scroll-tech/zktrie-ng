use crate::hash::{HashScheme, ZkHash, HASH_SIZE};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use strum::Display;
use NodeType::*;

mod imp;

mod rkyv_imp;
use crate::hash::poseidon::Poseidon;
pub use rkyv_imp::{
    ArchivedBranchNode, ArchivedLeafNode, ArchivedNode, IBranchNode, ILeafNode, INode, NodeViewer,
};

#[cfg(test)]
mod tests;

/// The magic bytes for the zkTrie node proof.
pub const MAGIC_NODE_BYTES: &[u8] = b"THIS IS SOME MAGIC BYTES FOR SMT m1rRXgP2xpDI";

/// NodeType is the type of node in the merkle tree.
///
/// Note there are some legacy types are not used anymore:
/// - `Parent` (0) => replaced by BranchL\*R\*
/// - `Leaf` (1) => replaced by `Leaf` (4)
/// - `Empty` (2) => replaced by `Empty` (5)
/// - `DBEntryTypeRoot` (3)
#[derive(Copy, Clone, Debug, Display, FromPrimitive, PartialEq)]
#[repr(u8)]
pub enum NodeType {
    /// Leaf node
    Leaf = 4,
    /// Empty node
    Empty = 5,

    /// Branch node for both child are terminal nodes.
    BranchLTRT = 6,
    /// branch node for left child is terminal node and right child is branch node.
    BranchLTRB = 7,
    /// branch node for left child is branch node and right child is terminal.
    BranchLBRT = 8,
    /// branch node for both child are branch nodes.
    BranchLBRB = 9,
}

/// A reference to another branch node that the node hash may not be calculated yet.
#[derive(Clone, Debug)]
pub struct LazyBranchHash {
    pub(crate) index: usize,
    pub(crate) resolved: Arc<OnceCell<ZkHash>>,
}

/// A lazy hash wrapper may be resolved later.
#[derive(Clone)]
pub enum LazyNodeHash {
    /// A node hash that is already calculated.
    Hash(ZkHash),
    /// A reference to another branch node that the node hash may not be calculated yet.
    LazyBranch(LazyBranchHash),
}

/// Leaf node can hold key-values.
///
/// The `value_hash` is computed by [`HashScheme::hash_bytes_array`].
#[derive(Clone)]
pub struct LeafNode {
    /// The node's key stored in a leaf node.
    node_key: ZkHash,
    /// The original key value that derives the node_key, kept here only for proof
    node_key_preimage: Option<[u8; 32]>,
    /// Store at most 256 `[u8; 32]` values as fields (represented by big endian integer),
    /// and the first 24 can be compressed (each 32 bytes consider as 2 fields),
    /// in hashing the compressed elements would be calculated first
    value_preimages: Vec<[u8; 32]>,
    /// use each bit for indicating the compressed flag for the first 24 fields
    compress_flags: u32,
    /// The hash of `value_preimages`.
    value_hash: OnceCell<ZkHash>,
}

/// A node could have two children.
#[derive(Clone, Debug)]
pub struct BranchNode {
    /// Type of this node.
    node_type: NodeType,
    /// Left child hash, defaults to be zero.
    child_left: LazyNodeHash,
    /// Right child hash, defaults to be zero.
    child_right: LazyNodeHash,
}

/// Three kinds of nodes in the merkle tree.
#[derive(Clone)]
pub enum NodeKind {
    /// An empty node.
    Empty,
    /// A leaf node.
    Leaf(LeafNode),
    /// A branch node.
    Branch(BranchNode),
}

/// Node struct represents a node in the merkle tree.
///
/// It's read-only and immutable, and the data is stored in `NodeKind`. Clone is cheap.
///
/// The `node_hash` is computed by [`HashScheme::hash`]:
/// - For `Leaf` node, it's computed by the hash of `Leaf` type and `[node_key, value_hash]`.
/// - For `Branch` node, it's computed by the hash of `Branch` type and `[child_left, child_right]`.
#[derive(Clone)]
pub struct Node<H = Poseidon> {
    /// nodeHash is the cache of the hash of the node to avoid recalculating
    pub(crate) node_hash: Arc<OnceCell<ZkHash>>,
    /// The data of the node.
    pub(crate) data: Arc<NodeKind>,
    _hash_scheme: std::marker::PhantomData<H>,
}

/// Errors that can occur when parsing a node.
#[derive(Debug, thiserror::Error)]
pub enum ParseNodeError<E> {
    /// Unexpected end, expected to read more bytes
    #[error("Expected at least {1} bytes, but only {0} bytes left")]
    Eof(usize, usize),
    /// Invalid node type, may occur when reading legacy data
    #[error("Invalid node type: {0}, are you reading legacy data?")]
    InvalidNodeType(u8),
    /// Error when hashing
    #[error(transparent)]
    HashError(E),
}
