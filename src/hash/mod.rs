//! Traits, helpers, and type definitions for hashing.

use alloy_primitives::FixedBytes;
use std::fmt::Debug;

pub mod poseidon;

pub mod key_hasher;

/// The size of an element in the hash scheme.
pub const HASH_SIZE: usize = 32;

const HASH_DOMAIN_ELEMS_BASE: u64 = 256;

/// A 32-byte big endian hash.
pub type ZkHash = FixedBytes<HASH_SIZE>;

/// The trait for hashing output.
#[must_use]
pub trait HashOutput: Copy + Clone + Sized {
    /// Convert the output into 32-byte big-endian array.
    fn as_canonical_repr(&self) -> ZkHash;

    /// Convert the 32-byte big-endian array into the output.
    fn from_canonical_repr(repr: ZkHash) -> Option<Self>;
}

/// HashScheme is a trait that defines how to hash two 32-byte arrays with a domain.
pub trait HashScheme: Debug + Copy + Clone + Sized {
    /// The error type for hashing.
    type Error: std::error::Error;

    /// Try to convert a byte array to a [`ZkHash`].
    fn new_hash_try_from_bytes(bytes: &[u8]) -> Result<ZkHash, Self::Error>;

    /// Hashes two `[u8; ELEMENT_SIZE]` with an u64 kind.
    ///
    /// This method treats input as little-endian.
    ///
    /// e.g. In poseidon implementation, it's treated as bn254 field element representation.
    ///
    /// The output of this method should be treated as an opaque type that can be converted to
    /// [`ZkHash`] with [`HashOutput::as_canonical_repr`].
    ///
    /// e.g. It could be a field element in poseidon implementation.
    fn raw_hash(kind: u64, le_bytes: [[u8; HASH_SIZE]; 2]) -> Result<impl HashOutput, Self::Error>;

    /// Hash two [`ZkHash`] with a domain.
    fn hash(kind: u64, inputs: [ZkHash; 2]) -> Result<ZkHash, Self::Error> {
        let le_bytes = inputs.map(|h| {
            let mut h: [u8; HASH_SIZE] = h.into();
            h.reverse();
            h
        });
        Self::raw_hash(kind, le_bytes).map(|h| h.as_canonical_repr())
    }

    /// Hash a variable length byte array with maximum length of `ELEMENT_SIZE`.
    fn hash_bytes(v: &[u8]) -> Result<ZkHash, Self::Error>;

    /// Hash an array of 32 bytes values with a compression bit flag.
    ///
    /// The first 24 values can be compressed (consider as hash).
    ///
    /// # Panics
    ///
    /// Panics if `value_bytes` is empty.
    fn hash_bytes_array(
        value_bytes: &[[u8; 32]],
        compression_flag: u32,
    ) -> Result<ZkHash, Self::Error> {
        assert!(!value_bytes.is_empty());
        let mut hashes = Vec::with_capacity(value_bytes.len());
        for (i, bytes) in value_bytes.iter().enumerate() {
            if i <= 24 && compression_flag & (1 << i) != 0 {
                hashes.push(Self::hash_bytes(bytes.as_slice())?);
            } else {
                hashes.push(Self::new_hash_try_from_bytes(bytes)?);
            }
        }

        let domain = value_bytes.len() as u64 * HASH_DOMAIN_ELEMS_BASE;
        while hashes.len() > 1 {
            let length = hashes.len();
            for i in 0..length / 2 {
                hashes[i] = Self::hash(domain, [hashes[2 * i], hashes[2 * i + 1]])?;
            }
            if length % 2 != 0 {
                hashes[length / 2] = hashes.pop().unwrap();
            }
            hashes.truncate(length / 2 + length % 2);
        }

        Ok(hashes[0])
    }
}
