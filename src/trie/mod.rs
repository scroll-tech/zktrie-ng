//! Traits, helpers, and type definitions for trie.

mod node;
pub use node::*;

mod zktrie;
pub use zktrie::*;

/// A trait for types that can be encoded into value bytes.
pub trait EncodeValueBytes {
    /// Encode the values into bytes.
    fn encode_values_bytes(&self) -> (Vec<[u8; 32]>, u32);
}

/// A trait for types that can be decoded from value bytes.
pub trait DecodeValueBytes<const LEN: usize> {
    /// Decode the values from bytes.
    fn decode_values_bytes(values: &[[u8; 32]; LEN]) -> Self;
}

impl<const LEN: usize> DecodeValueBytes<LEN> for [[u8; 32]; LEN] {
    fn decode_values_bytes(values: &[[u8; 32]; LEN]) -> Self {
        *values
    }
}
