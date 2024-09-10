use crate::hash::{HashOutput, HashScheme, Poseidon};
use poseidon_bn254::{hash_with_domain, Field, Fr, PrimeField};
use rand::{random, thread_rng};
use zktrie::HashField;
use zktrie_rust::hash::Hash as _;

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
    for _ in 0..100 {
        let kind: u64 = random();
        let a = Fr::random(thread_rng()).as_canonical_repr();
        let b = Fr::random(thread_rng()).as_canonical_repr();

        let out = Poseidon::hash(kind, [a, b]).unwrap();
        let expected = HashField::simple_hash_scheme(a.into(), b.into(), kind);
        assert_eq!(out.as_slice(), expected.as_ref());
    }
}
