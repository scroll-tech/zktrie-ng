//! Poseidon bn254 hash scheme.
use super::{HashOutput, HashScheme, ZkHash, HASH_DOMAIN_ELEMS_BASE, HASH_SIZE};
use poseidon_bn254::{hash_with_domain, Fr, PrimeField};

#[cfg(test)]
pub(crate) mod tests;

/// The length of a Poseidon hash.
pub const POSEIDON_HASH_LENGTH: usize = 32;

const HASH_DOMAIN_BYTE32: u64 = 2 * HASH_DOMAIN_ELEMS_BASE;

/// NODE_KEY_VALID_BYTES is the number of least significant bytes in the node key
/// that are considered valid to addressing the leaf node, and thus limits the
/// maximum trie depth to NODE_KEY_VALID_BYTES * 8.
/// We need to truncate the node key because the key is the output of Poseidon
/// hash and the key space doesn't fully occupy the range of power of two. It can
/// lead to an ambiguous bit representation of the key in the finite field
/// causing a soundness issue in the zk circuit.
pub const NODE_KEY_VALID_BYTES: u32 = 31;

/// The maximum trie depth.
const TRIE_MAX_LEVELS: usize = (NODE_KEY_VALID_BYTES * 8) as usize;

/// The Poseidon hash scheme.
#[derive(Default, Copy, Clone, Debug)]
pub struct Poseidon;

/// The error type for Poseidon hash.
#[derive(Copy, Clone, Debug, thiserror::Error)]
pub enum PoseidonError {
    /// The input is invalid as a field element.
    #[error("input is invalid as a field element")]
    InvalidFieldElement,
    /// Try to hash more than `HASH_SIZE` bytes.
    #[error(
        "hash_bytes can only hash up to {} bytes, but got {0} bytes",
        HASH_SIZE
    )]
    InvalidByteLength(usize),
}

impl HashOutput for Fr {
    #[inline]
    fn as_canonical_repr(&self) -> ZkHash {
        let mut bytes = self.to_repr();
        bytes.reverse();
        ZkHash::from(bytes)
    }

    #[inline]
    fn from_canonical_repr(repr: ZkHash) -> Option<Self> {
        let mut bytes: [u8; HASH_SIZE] = repr.into();
        bytes.reverse();
        Fr::from_repr_vartime(bytes)
    }
}

impl HashScheme for Poseidon {
    const TRIE_MAX_LEVELS: usize = TRIE_MAX_LEVELS;

    type Error = PoseidonError;

    fn new_hash_try_from_bytes(bytes: &[u8]) -> Result<ZkHash, Self::Error> {
        if bytes.len() > HASH_SIZE {
            Err(PoseidonError::InvalidByteLength(bytes.len()))
        } else {
            let padding = HASH_SIZE - bytes.len();
            let mut h = [0u8; HASH_SIZE];
            h[padding..].copy_from_slice(bytes);
            {
                if Fr::from_canonical_repr(h.into()).is_none() {
                    return Err(PoseidonError::InvalidFieldElement);
                }
            }
            Ok(ZkHash::from(h))
        }
    }

    fn raw_hash(kind: u64, le_bytes: [[u8; HASH_SIZE]; 2]) -> Result<impl HashOutput, Self::Error> {
        let a = Fr::from_repr_vartime(le_bytes[0]).ok_or(PoseidonError::InvalidFieldElement)?;
        let b = Fr::from_repr_vartime(le_bytes[1]).ok_or(PoseidonError::InvalidFieldElement)?;
        let domain = Fr::from(kind);
        Ok(hash_with_domain(&[a, b], domain))
    }

    fn hash_bytes(v: &[u8]) -> Result<ZkHash, Self::Error> {
        if v.len() > HASH_SIZE {
            return Err(PoseidonError::InvalidByteLength(v.len()));
        }
        const HALF_LEN: usize = HASH_SIZE / 2;

        let mut v_lo = [0u8; HASH_SIZE];
        let mut v_hi = [0u8; HASH_SIZE];
        if v.len() > HALF_LEN {
            v_lo[HALF_LEN..].copy_from_slice(&v[..HALF_LEN]);
            v_hi[HALF_LEN..v.len()].copy_from_slice(&v[HALF_LEN..]);
        } else {
            v_lo[HALF_LEN..HALF_LEN + v.len()].copy_from_slice(v);
        }

        Self::hash(HASH_DOMAIN_BYTE32, [v_lo.into(), v_hi.into()])
    }
}
