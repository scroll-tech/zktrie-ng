//! Keccak256 hash scheme.
use super::{HashOutput, HashScheme, ZkHash, HASH_SIZE};
use tiny_keccak::Hasher;

/// The length of a Poseidon hash.
pub const KECCAK_HASH_LENGTH: usize = 32;

/// The maximum trie depth.
const TRIE_MAX_LEVELS: usize = KECCAK_HASH_LENGTH * 8;

/// The Keccak hash scheme.
#[derive(Default, Copy, Clone, Debug)]
pub struct Keccak;

/// The error type for Poseidon hash.
#[derive(Copy, Clone, Debug, thiserror::Error)]
pub enum KeccakError {
    /// Try to hash more than `HASH_SIZE` bytes.
    #[error(
        "hash_bytes can only hash up to {} bytes, but got {0} bytes",
        HASH_SIZE
    )]
    InvalidByteLength(usize),
}

impl HashScheme for Keccak {
    const TRIE_MAX_LEVELS: usize = TRIE_MAX_LEVELS;

    type Error = KeccakError;

    fn new_hash_try_from_bytes(bytes: &[u8]) -> Result<ZkHash, Self::Error> {
        if bytes.len() > HASH_SIZE {
            Err(KeccakError::InvalidByteLength(bytes.len()))
        } else {
            let padding = HASH_SIZE - bytes.len();
            let mut h = [0u8; HASH_SIZE];
            h[padding..].copy_from_slice(bytes);
            Ok(ZkHash::from(h))
        }
    }

    fn raw_hash(kind: u64, le_bytes: [[u8; HASH_SIZE]; 2]) -> Result<impl HashOutput, Self::Error> {
        let mut hasher = tiny_keccak::Keccak::v256();
        let mut buf = [0u8; KECCAK_HASH_LENGTH];
        hasher.update(&kind.to_le_bytes());
        hasher.update(&le_bytes[0]);
        hasher.update(&le_bytes[1]);
        hasher.finalize(&mut buf);
        Ok(ZkHash::new(buf))
    }

    fn hash_bytes(v: &[u8]) -> Result<ZkHash, Self::Error> {
        if v.len() > HASH_SIZE {
            return Err(KeccakError::InvalidByteLength(v.len()));
        }
        let mut hasher = tiny_keccak::Keccak::v256();
        let mut buf = [0u8; KECCAK_HASH_LENGTH];
        hasher.update(v);
        hasher.finalize(&mut buf);
        Ok(ZkHash::new(buf))
    }
}
