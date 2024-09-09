use alloy_primitives::FixedBytes;

mod poseidon;
pub use poseidon::*;

#[cfg(test)]
mod tests;

/// The size of an element in the hash scheme.
pub const HASH_SIZE: usize = 32;

const HASH_DOMAIN_ELEMS_BASE: u64 = 256;

pub type ZkHash = FixedBytes<HASH_SIZE>;

/// The trait for hashing output.
pub trait HashOutput: Copy + Clone + Sized {
    /// Convert the output into 32-byte big-endian array.
    fn as_canonical_repr(&self) -> ZkHash;
}


/// HashScheme is a trait that defines how to hash two 32-byte arrays with a domain.
pub trait HashScheme {
    /// The error type for hashing.
    type Error;

    /// Try to convert a byte array to a [`ZkHash`].
    fn new_hash_try_from_bytes(bytes: &[u8]) -> Result<ZkHash, Self::Error>;

    /// Hashes two `[u8; ELEMENT_SIZE]` with an u64 kind.
    ///
    /// This method treats input as little-endian.
    ///
    /// e.g. In poseidon implementation, it's treated as bn254 field element representation.
    ///
    /// The output of this method should be treated as an opaque type that can be converted to
    /// [`ZkHash`] with [`HashOutput::to_canonical_repr`].
    ///
    /// e.g. It could be a field element in poseidon implementation.
    fn raw_hash(
        kind: u64,
        le_bytes: [[u8; HASH_SIZE]; 2],
    ) -> Result<impl HashOutput, Self::Error>;

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
    fn hash_bytes_array(value_bytes: &[[u8; 32]], compression_flags: u32) -> Result<ZkHash, Self::Error> {
        assert!(!value_bytes.len() > 1);
        let mut hashes = Vec::with_capacity(value_bytes.len());
        for (i, byte) in value_bytes.iter().enumerate() {
            if compression_flags & (1 << i) != 0 {
                hashes.push(Self::hash_bytes(byte.as_slice())?);
            } else {
                hashes.push(Self::new_hash_try_from_bytes(byte)?);
            }
        }

        let domain = value_bytes.len() as u64 * HASH_DOMAIN_ELEMS_BASE;
        while hashes.len() > 1 {
            let mut out = Vec::new();
            for pair in hashes.chunks(2) {
                out.push(if pair.len() == 2 {
                    Self::hash(domain, [pair[0], pair[1]])?
                } else {
                    pair[0]
                });
            }
            hashes = out;
        }

        Ok(hashes[0])
    }
}