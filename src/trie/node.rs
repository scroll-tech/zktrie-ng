use crate::hash::{HashScheme, ZkHash, HASH_SIZE};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use strum::Display;
use NodeType::*;

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

/// Leaf node in the merkle tree.
///
/// The `value_hash` is computed by [`HashScheme::hash_bytes_array`].
#[derive(Clone, Debug)]
pub struct LeafNode {
    /// The node's key stored in a leaf node.
    node_key: ZkHash,
    /// The original key value that derives the node_key, kept here only for proof
    node_key_preimage: Option<[u8; 32]>,
    /// Store at most 256 `[u8; 32]` values as fields (represented by big endian integer),
    /// and the first 24 can be compressed (each bytes32 consider as 2 fields), in hashing the compressed
    /// elemments would be calculated first
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

impl<H: HashScheme> Node<H> {
    /// Empty node.
    pub const EMPTY: Node<H> = Node {
        node_hash: ZkHash::ZERO,
        data: NodeKind::Empty,
        _hash_scheme: std::marker::PhantomData,
    };

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
            node_hash,
            data: NodeKind::Leaf(LeafNode {
                node_key,
                node_key_preimage,
                value_preimages,
                compress_flags,
                value_hash,
            }),
            _hash_scheme: std::marker::PhantomData,
        })
    }

    /// Create a new branch node.
    pub fn new_branch(
        node_type: NodeType,
        child_left: ZkHash,
        child_right: ZkHash,
    ) -> Result<Self, H::Error> {
        let node_hash = H::hash(node_type as u64, [child_left, child_right])?;

        Ok(Node {
            node_hash,
            data: NodeKind::Branch(BranchNode {
                node_type,
                child_left,
                child_right,
            }),
            _hash_scheme: std::marker::PhantomData,
        })
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


    /// check if the node is 'terminated', i.e. empty or leaf node
    #[inline]
    pub fn is_terminal(&self) -> bool {
        self.node_type().is_terminal()
    }

    /// Get the node hash.
    #[inline]
    pub fn node_hash(&self) -> &ZkHash {
        &self.node_hash
    }

    /// Try as a leaf node.
    pub fn as_leaf(&self) -> Option<&LeafNode> {
        match &self.data {
            NodeKind::Leaf(leaf) => Some(leaf),
            _ => None,
        }
    }

    /// Try as a branch node.
    pub fn as_branch(&self) -> Option<&BranchNode> {
        match &self.data {
            NodeKind::Branch(branch) => Some(branch),
            _ => None,
        }
    }
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
                Self::new_branch(node_type, child_left, child_right).map_err(HashError)
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

                Self::new_leaf(node_key, value_preimages, compress_flags, node_key_preimage)
                    .map_err(HashError)
            }
            Empty => Ok(Self::EMPTY),
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

    /// Get the `value_hash`.
    #[inline]
    pub fn value_hash(&self) -> ZkHash {
        self.value_hash
    }
}

impl BranchNode {
    /// Get the left child hash.
    #[inline]
    pub fn child_left(&self) -> ZkHash {
        self.child_left
    }

    /// Get the right child hash.
    #[inline]
    pub fn child_right(&self) -> ZkHash {
        self.child_right
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

#[cfg(test)]
mod tests {
    use zktrie::HashField;
    use zktrie_rust::hash::AsHash;
    use zktrie_rust::types::Hashable;
    use crate::hash::Poseidon;
    use super::*;

    type OldNode = zktrie_rust::types::Node::<AsHash<HashField>>;

    #[test]
    fn test_empty_node() {
        let expected = OldNode::new_empty_node();
        let node_hash = expected.calc_node_hash().unwrap().node_hash().unwrap();

        assert_eq!(Node::<Poseidon>::EMPTY.node_hash, node_hash.as_ref());
    }

    #[test]
    fn test_leaf_node() {
        let expected = OldNode::new_leaf_node(
            AsHash::from_bytes(&[1u8; 32]).unwrap(),
            0,
            vec![[2u8; 32]],
        );
        let node_hash = expected.calc_node_hash().unwrap().node_hash().unwrap();

        let node = Node::<Poseidon>::new_leaf(
            Poseidon::new_hash_try_from_bytes(&[1u8; 32]).unwrap(),
            vec![[2u8; 32]],
            0,
            None,
        ).unwrap();

        assert_eq!(node.node_hash, node_hash.as_ref());
    }

    #[test]
    fn test_branch_node() {
        let expected = OldNode::new_parent_node(
            zktrie_rust::types::NodeType::NodeTypeBranch0,
            AsHash::from_bytes(&[1u8; 32]).unwrap(),
            AsHash::from_bytes(&[2u8; 32]).unwrap(),
        );
        let node_hash = expected.calc_node_hash().unwrap().node_hash().unwrap();

        let node = Node::<Poseidon>::new_branch(
            BranchLTRT,
            Poseidon::new_hash_try_from_bytes(&[1u8; 32]).unwrap(),
            Poseidon::new_hash_try_from_bytes(&[2u8; 32]).unwrap(),
        ).unwrap();

        assert_eq!(node.node_hash, node_hash.as_ref());
    }
}