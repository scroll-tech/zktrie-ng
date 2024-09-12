#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Error {
    // InvalidField is key not inside the finite field.
    InvalidField,
    // NodeKeyAlreadyExists is used when a node key already exists.
    NodeKeyAlreadyExists,
    // KeyNotFound is used when a key is not found in the ZkTrieImpl.
    KeyNotFound,
    // NodeBytesBadSize is used when the data of a node has an incorrect
    // size and can't be parsed.
    NodeBytesBadSize,
    // ReachedMaxLevel is used when a traversal of the MT reaches the
    // maximum level.
    MaxLevelReached,
    // InvalidNodeFound is used when an invalid node is found and can't
    // be parsed.
    InvalidNodeFound,
    // InvalidProofBytes is used when a serialized proof is invalid.
    InvalidProofBytes,
    // EntryIndexAlreadyExists is used when the entry index already
    // exists in the tree.
    EntryIndexAlreadyExists,
    // NotWritable is used when the ZkTrieImpl is not writable and a
    // write     pub fntion is called
    NotWritable,
}
