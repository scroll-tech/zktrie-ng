#![allow(missing_docs)]
use criterion::{criterion_group, criterion_main, Criterion};
use poseidon_bn254::{hash_with_domain, Fr, PrimeField};
use rand::prelude::*;
use zktrie::HashField;
use zktrie_ng::trie::NodeType;
use zktrie_ng::{
    hash::{poseidon::Poseidon, HashScheme},
    trie::Node,
};
use zktrie_rust::hash::AsHash;

type OldNode = zktrie_rust::types::Node<AsHash<HashField>>;

fn bench_parse_node_inner(c: &mut Criterion, name: &str, node_bytes: Vec<u8>) {
    let mut group = c.benchmark_group(name);
    group.bench_with_input("zktrie-ng", &node_bytes, |b, node_bytes| {
        b.iter(|| {
            let node = Node::<Poseidon>::try_from(node_bytes.as_slice()).unwrap();
            *node.get_or_calculate_node_hash().unwrap()
        });
    });
    group.bench_with_input("zktrie", &node_bytes, |b, node_bytes| {
        b.iter(|| {
            OldNode::new_node_from_bytes(&node_bytes)
                .unwrap()
                .calc_node_hash()
                .unwrap()
                .node_hash()
                .unwrap()
        });
    });
    group.finish();
}

fn bench_parse_node(c: &mut Criterion) {
    let mut rng = SmallRng::seed_from_u64(42);

    let account_leaf = {
        let key: [u8; 20] = rng.gen();
        let values: [[u8; 32]; 5] = rng.gen();
        Node::<Poseidon>::new_leaf(
            Poseidon::hash_bytes(&key).unwrap(),
            values.to_vec(),
            0b11111,
            None,
        )
        .unwrap()
    };
    bench_parse_node_inner(c, "Parse Account Node", account_leaf.canonical_value(false));

    let storage_leaf = {
        let key: [u8; 32] = rng.gen();
        let values: [[u8; 32]; 1] = rng.gen();
        Node::<Poseidon>::new_leaf(
            Poseidon::hash_bytes(&key).unwrap(),
            values.to_vec(),
            0b1,
            None,
        )
        .unwrap()
    };

    bench_parse_node_inner(c, "Parse Storage Node", storage_leaf.canonical_value(false));

    let branch_node = Node::<Poseidon>::new_branch(
        NodeType::BranchLTRT,
        *account_leaf.get_or_calculate_node_hash().unwrap(),
        *storage_leaf.get_or_calculate_node_hash().unwrap(),
    );

    bench_parse_node_inner(c, "Parse Branch Node", branch_node.canonical_value(false));
}

fn poseidon_hash_scheme(a: &[u8; 32], b: &[u8; 32], domain: &[u8; 32]) -> Option<[u8; 32]> {
    let a = Fr::from_repr_vartime(*a)?;
    let b = Fr::from_repr_vartime(*b)?;
    let domain = Fr::from_repr_vartime(*domain)?;
    Some(hash_with_domain(&[a, b], domain).to_repr())
}

fn criterion_benchmark(c: &mut Criterion) {
    zktrie::init_hash_scheme_simple(poseidon_hash_scheme);
    bench_parse_node(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
