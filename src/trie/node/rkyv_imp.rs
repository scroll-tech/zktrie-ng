#![allow(missing_debug_implementations)]
use super::*;
use alloy_primitives::bytes::Bytes;
use rkyv::{
    bytecheck::StructCheckError, out_field, Archive, Archived, CheckBytes, Deserialize, Fallible,
    Serialize,
};
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomData;

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

/// An archived [`LeafNode`].
#[derive(Debug)]
pub struct ArchivedLeafNode {
    pub(crate) node_key: Archived<ZkHash>,
    pub(crate) node_key_preimage: Archived<Option<[u8; 32]>>,
    pub(crate) value_preimages: Archived<Vec<[u8; 32]>>,
    pub(crate) compress_flags: Archived<u32>,
    pub(crate) value_hash: Archived<Option<ZkHash>>,
}

/// A leaf node that may be archived.
#[derive(Clone, Debug)]
pub enum ILeafNode<'a> {
    /// Owned leaf node.
    Owned(&'a LeafNode),
    /// Archived leaf node.
    Archived(&'a ArchivedLeafNode),
}

/// A resolver for [`LeafNode`].
pub struct LeafNodeResolver {
    node_key: rkyv::Resolver<ZkHash>,
    node_key_preimage: rkyv::Resolver<Option<[u8; 32]>>,
    value_preimages: rkyv::Resolver<Vec<[u8; 32]>>,
    compress_flags: rkyv::Resolver<u32>,
    value_hash: rkyv::Resolver<Option<ZkHash>>,
}

/// An archived [`BranchNode`].
#[derive(Copy, Clone, Debug)]
pub struct ArchivedBranchNode {
    pub(crate) node_type: Archived<u8>,
    pub(crate) child_left: Archived<ZkHash>,
    pub(crate) child_right: Archived<ZkHash>,
}

/// A branch node that may be archived.
#[derive(Clone, Debug)]
pub enum IBranchNode<'a> {
    /// Owned branch node.
    Owned(&'a BranchNode),
    /// Archived branch node.
    Archived(&'a ArchivedBranchNode),
}

/// A resolver for [`BranchNode`].
pub struct BranchNodeResolver {
    node_type: rkyv::Resolver<u8>,
    child_left: rkyv::Resolver<ZkHash>,
    child_right: rkyv::Resolver<ZkHash>,
}

/// An archived [`Node`].
#[derive(Debug)]
pub struct ArchivedNode {
    node_hash: Archived<Option<ZkHash>>,
    data: Archived<NodeKind>,
}

/// A node that may be archived.
pub struct NodeResolver {
    node_hash: rkyv::Resolver<Option<ZkHash>>,
    data: rkyv::Resolver<NodeKind>,
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

impl Archive for LeafNode {
    type Archived = ArchivedLeafNode;
    type Resolver = LeafNodeResolver;

    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
        let (fp, fo) = out_field!(out.node_key);
        self.node_key.resolve(pos + fp, resolver.node_key, fo);
        let (fp, fo) = out_field!(out.node_key_preimage);
        self.node_key_preimage
            .resolve(pos + fp, resolver.node_key_preimage, fo);
        let (fp, fo) = out_field!(out.value_preimages);
        self.value_preimages
            .resolve(pos + fp, resolver.value_preimages, fo);
        let (fp, fo) = out_field!(out.compress_flags);
        self.compress_flags
            .resolve(pos + fp, resolver.compress_flags, fo);
        let (fp, fo) = out_field!(out.value_hash);
        self.value_hash
            .get()
            .copied()
            .resolve(pos + fp, resolver.value_hash, fo);
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for LeafNode
where
    [u8; 32]: Serialize<S>,
    Vec<[u8; 32]>: Serialize<S>,
{
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(LeafNodeResolver {
            node_key: Serialize::<S>::serialize(&self.node_key, serializer)?,
            node_key_preimage: Serialize::<S>::serialize(&self.node_key_preimage, serializer)?,
            value_preimages: Serialize::<S>::serialize(&self.value_preimages, serializer)?,
            compress_flags: Serialize::<S>::serialize(&self.compress_flags, serializer)?,
            value_hash: Serialize::<S>::serialize(&self.value_hash.get().copied(), serializer)?,
        })
    }
}

impl<D: Fallible + ?Sized> Deserialize<LeafNode, D> for Archived<LeafNode> {
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<LeafNode, D::Error> {
        Ok(LeafNode {
            node_key: Deserialize::<ZkHash, D>::deserialize(&self.node_key, deserializer)?,
            node_key_preimage: Deserialize::<Option<[u8; 32]>, D>::deserialize(
                &self.node_key_preimage,
                deserializer,
            )?,
            value_preimages: Deserialize::<Vec<[u8; 32]>, D>::deserialize(
                &self.value_preimages,
                deserializer,
            )?,
            compress_flags: Deserialize::<u32, D>::deserialize(&self.compress_flags, deserializer)?,
            value_hash: {
                let value_hash = OnceCell::new();
                if let Some(hash) =
                    Deserialize::<Option<ZkHash>, D>::deserialize(&self.value_hash, deserializer)?
                {
                    value_hash.set(hash).expect("infalible");
                }

                value_hash
            },
        })
    }
}

impl ArchivedLeafNode {
    /// Get the `node_key` stored in a leaf node.
    #[inline]
    pub fn node_key(&self) -> &ZkHash {
        &self.node_key
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
        self.compress_flags
    }

    /// Get the `value_hash` of the leaf node.
    #[inline]
    pub fn value_hash(&self) -> Option<&ZkHash> {
        self.value_hash.as_ref()
    }

    /// Get the `value_hash`
    #[inline]
    pub fn get_or_calc_value_hash<H: HashScheme>(&self) -> Result<ZkHash, H::Error> {
        match self.value_hash() {
            Some(hash) => Ok(*hash),
            None => H::hash_bytes_array(self.value_preimages(), self.compress_flags()),
        }
    }
}

impl ILeafNode<'_> {
    /// Get the `node_key` stored in a leaf node.
    #[inline]
    pub fn node_key(&self) -> &ZkHash {
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
    pub fn value_hash(&self) -> Option<&ZkHash> {
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

impl<C: ?Sized> CheckBytes<C> for ArchivedLeafNode
where
    Archived<Vec<[u8; 32]>>: CheckBytes<C>,
{
    type Error = StructCheckError;
    unsafe fn check_bytes<'c>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'c Self, StructCheckError> {
        Archived::<ZkHash>::check_bytes(&(*value).node_key, context).map_err(|e| {
            StructCheckError {
                field_name: "node_key",
                inner: Box::new(e),
            }
        })?;
        Archived::<Option<[u8; 32]>>::check_bytes(&(*value).node_key_preimage, context).map_err(
            |e| StructCheckError {
                field_name: "node_key_preimage",
                inner: Box::new(e),
            },
        )?;
        Archived::<Vec<[u8; 32]>>::check_bytes(&(*value).value_preimages, context).map_err(
            |e| StructCheckError {
                field_name: "value_preimages",
                inner: Box::new(e),
            },
        )?;
        Archived::<u32>::check_bytes(&(*value).compress_flags, context).map_err(|e| {
            StructCheckError {
                field_name: "compress_flags",
                inner: Box::new(e),
            }
        })?;
        Archived::<Option<ZkHash>>::check_bytes(&(*value).value_hash, context).map_err(|e| {
            StructCheckError {
                field_name: "value_hash",
                inner: Box::new(e),
            }
        })?;
        Ok(&*value)
    }
}

impl Archive for BranchNode {
    type Archived = ArchivedBranchNode;
    type Resolver = BranchNodeResolver;

    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
        let (fp, fo) = out_field!(out.node_type);
        (self.node_type as u8).resolve(pos + fp, resolver.node_type, fo);
        let (fp, fo) = out_field!(out.child_left);
        self.child_left
            .try_as_hash()
            .expect("cannot archive a lazy hash")
            .resolve(pos + fp, resolver.child_left, fo);
        let (fp, fo) = out_field!(out.child_right);
        self.child_right
            .try_as_hash()
            .expect("cannot archive a lazy hash")
            .resolve(pos + fp, resolver.child_right, fo);
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for BranchNode {
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(BranchNodeResolver {
            node_type: Serialize::<S>::serialize(&(self.node_type as u8), serializer)?,
            child_left: Serialize::<S>::serialize(self.child_left.unwrap_ref(), serializer)?,
            child_right: Serialize::<S>::serialize(self.child_right.unwrap_ref(), serializer)?,
        })
    }
}

impl<D: Fallible + ?Sized> Deserialize<BranchNode, D> for Archived<BranchNode> {
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<BranchNode, D::Error> {
        Ok(BranchNode {
            node_type: NodeType::from_u8(self.node_type).expect("invalid node type"),
            child_left: LazyNodeHash::Hash(Deserialize::<ZkHash, D>::deserialize(
                &self.child_left,
                deserializer,
            )?),
            child_right: LazyNodeHash::Hash(Deserialize::<ZkHash, D>::deserialize(
                &self.child_right,
                deserializer,
            )?),
        })
    }
}

impl<C: ?Sized> CheckBytes<C> for ArchivedBranchNode {
    type Error = StructCheckError;
    unsafe fn check_bytes<'c>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'c Self, StructCheckError> {
        Archived::<u8>::check_bytes(&(*value).node_type, context).map_err(|e| {
            StructCheckError {
                field_name: "node_type",
                inner: Box::new(e),
            }
        })?;
        Archived::<ZkHash>::check_bytes(&(*value).child_left, context).map_err(|e| {
            StructCheckError {
                field_name: "child_left",
                inner: Box::new(e),
            }
        })?;
        Archived::<ZkHash>::check_bytes(&(*value).child_right, context).map_err(|e| {
            StructCheckError {
                field_name: "child_right",
                inner: Box::new(e),
            }
        })?;
        Ok(&*value)
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
        LazyNodeHash::Hash(self.child_left)
    }

    /// Get the right child hash.
    #[inline]
    pub fn child_right(&self) -> LazyNodeHash {
        LazyNodeHash::Hash(self.child_right)
    }

    /// As the parts
    #[inline]
    pub fn as_parts(&self) -> (NodeType, LazyNodeHash, LazyNodeHash) {
        (self.node_type(), self.child_left(), self.child_right())
    }
}

impl<H> Archive for Node<H> {
    type Archived = ArchivedNode;
    type Resolver = NodeResolver;

    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
        let (fp, fo) = out_field!(out.node_hash);
        self.node_hash
            .get()
            .copied()
            .resolve(pos + fp, resolver.node_hash, fo);
        let (fp, fo) = out_field!(out.data);
        self.data.as_ref().resolve(pos + fp, resolver.data, fo);
    }
}

impl<S: Fallible + ?Sized, H> Serialize<S> for Node<H>
where
    LeafNode: Serialize<S>,
    BranchNode: Serialize<S>,
{
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(NodeResolver {
            node_hash: Serialize::<S>::serialize(&self.node_hash.get().copied(), serializer)?,
            data: Serialize::<S>::serialize(self.data.as_ref(), serializer)?,
        })
    }
}

impl<D: Fallible + ?Sized, H> Deserialize<Node<H>, D> for Archived<Node<H>> {
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<Node<H>, D::Error> {
        Ok(Node {
            node_hash: {
                let node_hash = OnceCell::new();
                if let Some(hash) =
                    Deserialize::<Option<ZkHash>, D>::deserialize(&self.node_hash, deserializer)?
                {
                    node_hash.set(hash).expect("infalible");
                }

                Arc::new(node_hash)
            },
            data: Arc::new(Deserialize::<NodeKind, D>::deserialize(
                &self.data,
                deserializer,
            )?),
            _hash_scheme: PhantomData,
        })
    }
}

impl<C: ?Sized> CheckBytes<C> for ArchivedNode
where
    Archived<NodeKind>: CheckBytes<C>,
{
    type Error = StructCheckError;
    unsafe fn check_bytes<'c>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'c Self, StructCheckError> {
        Archived::<Option<ZkHash>>::check_bytes(&(*value).node_hash, context).map_err(|e| {
            StructCheckError {
                field_name: "node_hash",
                inner: Box::new(e),
            }
        })?;
        Archived::<NodeKind>::check_bytes(&(*value).data, context).map_err(|e| {
            StructCheckError {
                field_name: "data",
                inner: Box::new(e),
            }
        })?;
        Ok(&*value)
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
                    1 + HASH_SIZE + size_of::<u32>() + 32 * leaf.value_preimages.len() + 1,
                );
                bytes.push(Leaf as u8);
                bytes.extend_from_slice(leaf.node_key.as_ref());
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
                bytes.push(branch.node_type as u8);
                bytes.extend_from_slice(branch.child_left.as_ref());
                bytes.extend_from_slice(branch.child_right.as_ref());
                bytes
            }
            ArchivedNodeKind::Empty => {
                vec![Empty as u8]
            }
        }
    }
}

impl NodeViewer {
    /// View the archived node.
    pub fn view(&self) -> &ArchivedNode {
        // SAFETY: The bytes are guaranteed to be a valid archived node
        unsafe { rkyv::archived_root::<Node>(self.data.as_ref()) }
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

impl<H: HashScheme> Debug for INode<H> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            INode::Owned(node) => Debug::fmt(node, f),
            INode::Archived(node) => Debug::fmt(node.view(), f),
        }
    }
}
