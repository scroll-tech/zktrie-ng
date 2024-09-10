use crate::hash::{HashScheme, ZkHash, HASH_SIZE};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use strum::Display;
use NodeType::*;

mod imp;

#[cfg(test)]
mod tests;

/// NodeType is the type of node in the merkle tree.
///
/// Note there are some legacy types are not used anymore:
/// - `Parent` (0) => replaced by BranchL\*R\*
/// - `Leaf` (1) => replaced by `Leaf` (4)
/// - `Empty` (2) => replaced by `Empty` (5)
/// - `DBEntryTypeRoot` (3)
#[derive(Copy, Clone, Debug, Display, FromPrimitive, PartialEq)]
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

impl NodeType {
    /// check if the node is 'terminated', i.e. empty or leaf node
    #[inline(always)]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Leaf | Empty)
    }
}

/// Leaf node can hold key-values.
///
/// The `value_hash` is computed by [`HashScheme::hash_bytes_array`].
#[derive(Clone, Debug)]
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
    value_hash: ZkHash,
}

/// A node could have two children.
#[derive(Clone, Debug)]
pub struct BranchNode {
    /// Type of this node.
    node_type: NodeType,
    /// Left child hash, defaults to be zero.
    child_left: ZkHash,
    /// Right child hash, defaults to be zero.
    child_right: ZkHash,
}

/// Three kinds of nodes in the merkle tree.
#[derive(Clone, Debug)]
pub enum NodeKind {
    Empty,
    Leaf(LeafNode),
    Branch(BranchNode),
}

/// Node struct represents a node in the merkle tree.
///
/// The `node_hash` is computed by [`HashScheme::hash`]:
/// - For `Leaf` node, it's computed by the hash of `Leaf` type and `[node_key, value_hash]`.
/// - For `Branch` node, it's computed by the hash of `Branch` type and `[child_left, child_right]`.
#[derive(Clone, Debug)]
pub struct Node<H> {
    /// nodeHash is the cache of the hash of the node to avoid recalculating
    node_hash: ZkHash,
    /// The data of the node.
    data: NodeKind,
    _hash_scheme: std::marker::PhantomData<H>,
}

/// Errors that can occur when parsing a node.
#[derive(Debug, thiserror::Error)]
pub enum ParseNodeError<E> {
    #[error("Expected at least {1} bytes, but only {0} bytes left")]
    Eof(usize, usize),
    #[error("Invalid node type: {0}, are you reading legacy data?")]
    InvalidNodeType(u8),
    #[error(transparent)]
    HashError(E),
}