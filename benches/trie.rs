#![allow(missing_docs)]

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use poseidon_bn254::{hash_with_domain, Fr, PrimeField};
use rand::prelude::*;
use std::hint::black_box;
use zktrie::HashField;
use zktrie_ng::db::HashMapDb;
use zktrie_ng::hash::key_hasher::NoCacheHasher;
use zktrie_ng::{
    hash::{poseidon::Poseidon, HashScheme},
    trie::ZkTrie,
};
use zktrie_rust::{db::SimpleDb, hash::AsHash, types::TrieHashScheme};

type NodeOld = zktrie_rust::types::Node<AsHash<HashField>>;
type TrieOld =
    zktrie_rust::raw::ZkTrieImpl<AsHash<HashField>, SimpleDb, { Poseidon::TRIE_MAX_LEVELS }>;

fn bench_trie_update(c: &mut Criterion) {
    let mut rng = SmallRng::seed_from_u64(42);
    let mut group = c.benchmark_group("Trie Update");

    let k: [u8; 20] = rng.gen();
    let values: [[u8; 32]; 5] = rng.gen();
    let values = values.to_vec();

    group.bench_with_input("zktrie-ng", &(k, values.clone()), |b, (k, values)| {
        b.iter_batched(
            || {
                let trie = ZkTrie::default();
                (trie, k, values.clone())
            },
            |(mut trie, k, values)| {
                trie.raw_update(k, values, black_box(0b11111)).unwrap();
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_with_input("zktrie", &(k, values), |b, (k, values)| {
        b.iter_batched(
            || {
                let trie = TrieOld::new_zktrie_impl(SimpleDb::new()).unwrap();
                (trie, k, values.clone())
            },
            |(mut trie, k, values)| {
                let key = NodeOld::hash_bytes(k).unwrap();
                trie.try_update(&key, black_box(0b11111), values).unwrap();
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_trie_operation(c: &mut Criterion) {
    let mut rng = SmallRng::seed_from_u64(42);

    let mut trie = ZkTrie::default();
    let mut trie_old = TrieOld::new_zktrie_impl(SimpleDb::new()).unwrap();

    let mut keys = vec![];

    for _ in 0..100 {
        let k: [u8; 20] = rng.gen();
        let values: [[u8; 32]; 5] = rng.gen();
        let values = values.to_vec();

        trie.raw_update(k, values.clone(), 0b11111).unwrap();
        let key = NodeOld::hash_bytes(&k).unwrap();
        trie_old.try_update(&key, 0b11111, values).unwrap();
        keys.push((Poseidon::hash_bytes(&k).unwrap(), key));
    }

    trie.commit().unwrap();
    trie_old.prepare_root().unwrap();
    trie_old.commit().unwrap();

    let mut group = c.benchmark_group("Trie Get");
    keys.shuffle(&mut rng);
    group.bench_with_input("zktrie-ng", &(&trie, &keys[..10]), |b, (trie, keys)| {
        b.iter(|| {
            keys.iter()
                .map(|(key, _)| trie.get_node_by_key(key).unwrap())
                .collect::<Vec<_>>()
        });
    });
    group.bench_with_input("zktrie", &(&trie_old, &keys[..10]), |b, (trie, keys)| {
        b.iter(|| {
            keys.iter()
                .map(|(_, key)| trie.try_get(key).unwrap())
                .collect::<Vec<_>>()
        });
    });
    group.finish();

    let mut group = c.benchmark_group("Trie Delete");
    keys.shuffle(&mut rng);
    group.bench_with_input("zktrie-ng", &(&trie, &keys[..10]), |b, (trie, keys)| {
        b.iter_batched(
            || {
                let root = *trie.root().unwrap_ref();
                let db = HashMapDb::from_map(false, trie.db().inner().clone());
                let trie = ZkTrie::<Poseidon>::new_with_root(db, NoCacheHasher, root).unwrap();
                (trie, keys)
            },
            |(mut trie, keys)| {
                for (key, _) in keys.iter() {
                    trie.delete_by_node_key(*key).unwrap();
                }
            },
            BatchSize::SmallInput,
        )
    });
    group.bench_with_input("zktrie", &(&trie_old, &keys[..10]), |b, (trie, keys)| {
        b.iter_batched(
            || {
                let root = trie.root();
                let db = trie.get_db().clone();
                let trie = TrieOld::new_zktrie_impl_with_root(db, root).unwrap();
                (trie, keys)
            },
            |(mut trie, keys)| {
                for (_, key) in keys.iter() {
                    trie.try_delete(key).unwrap();
                }
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

fn poseidon_hash_scheme(a: &[u8; 32], b: &[u8; 32], domain: &[u8; 32]) -> Option<[u8; 32]> {
    let a = Fr::from_repr_vartime(*a)?;
    let b = Fr::from_repr_vartime(*b)?;
    let domain = Fr::from_repr_vartime(*domain)?;
    Some(hash_with_domain(&[a, b], domain).to_repr())
}

fn criterion_benchmark(c: &mut Criterion) {
    zktrie::init_hash_scheme_simple(poseidon_hash_scheme);
    bench_trie_update(c);
    bench_trie_operation(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
