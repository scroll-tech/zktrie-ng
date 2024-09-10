use super::{HashOutput, HashScheme, Poseidon};
use poseidon_bn254::{hash_with_domain, Field, Fr, PrimeField};
use rand::{random, thread_rng, Rng};
use zktrie::HashField;
use zktrie_rust::hash::{AsHash, Hash as _};
use zktrie_rust::types::{Node, TrieHashScheme};

#[ctor::ctor]
fn set_hash_scheme() {
    zktrie::init_hash_scheme_simple(poseidon_hash_scheme)
}

fn poseidon_hash_scheme(a: &[u8; 32], b: &[u8; 32], domain: &[u8; 32]) -> Option<[u8; 32]> {
    let a = Fr::from_repr_vartime(*a)?;
    let b = Fr::from_repr_vartime(*b)?;
    let domain = Fr::from_repr_vartime(*domain)?;
    Some(hash_with_domain(&[a, b], domain).to_repr())
}

#[test]
fn test_hash() {
    for _ in 0..1000 {
        let kind: u64 = random();
        let a = Fr::random(thread_rng()).as_canonical_repr();
        let b = Fr::random(thread_rng()).as_canonical_repr();

        let out = Poseidon::hash(kind, [a, b]).unwrap();
        let expected = HashField::simple_hash_scheme(a.into(), b.into(), kind);
        assert_eq!(out.as_slice(), expected.as_ref());
    }
}

#[test]
fn test_hash_bytes() {
    for _ in 0..1000 {
        let n_bytes = thread_rng().gen_range(0..32);
        let bytes: Vec<u8> = (0..n_bytes).map(|_| random()).collect();
        let out = Poseidon::hash_bytes(&bytes).unwrap();
        let expected = Node::<AsHash<HashField>>::hash_bytes(&bytes).unwrap();
        assert_eq!(out.as_slice(), expected.as_ref());
    }
}

#[test]
fn test_hash_bytes_array() {
    for _ in 0..100 {
        let mut compression_flag: u32 = 0;
        let n_bytes: usize = thread_rng().gen_range(1..32) as usize;
        let mut bytes = Vec::with_capacity(n_bytes);
        for i in 0..24.min(n_bytes) {
            if random() {
                bytes.push(Fr::random(thread_rng()).as_canonical_repr().into());
            } else {
                bytes.push(random());
                compression_flag |= 1 << i;
            }
        }
        for _ in 24..n_bytes {
            bytes.push(Fr::random(thread_rng()).as_canonical_repr().into());
        }
        let out = Poseidon::hash_bytes_array(&bytes, compression_flag).unwrap();
        let expected = Node::<AsHash<HashField>>::handling_elems_and_bytes32(compression_flag, &bytes).unwrap();
        assert_eq!(out.as_slice(), expected.as_ref());
    }
}