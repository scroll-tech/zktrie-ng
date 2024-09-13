# zkTrie-ng

[![docs](https://img.shields.io/badge/docs-latest-blue)](https://scroll-tech.github.io/zktrie-ng/zktrie_ng/index.html)
![CI Status](https://img.shields.io/github/actions/workflow/status/scroll-tech/zktrie-ng/CI)

Full-featured zkTrie implementation in Rust.

## Example

```rust
use zktrie_ng::{
    trie,
    hash::{
        key_hasher::NoCacheHasher,
        poseidon::Poseidon,
    },
    db::{HashMapDb, SledDb},
};

// A ZkTrie using Poseidon hash scheme,
// HashMap as backend kv database and NoCacheHasher as key hasher.
type ZkTrie = trie::ZkTrie<Poseidon, HashMapDb, NoCacheHasher>;
// Or you can store the zkTrie data in a sled database.
// type ZkTrie = trie::ZkTrie<Poseidon, SledDb, NoCacheHasher>;

fn main() {
    let mut trie = ZkTrie::new(HashMapDb::new(), NoCacheHasher);
    // or this is default mode
    // let mut trie = ZkTrie::default();

    trie.raw_update(&[1u8; 32], vec![[1u8; 32]], 1).unwrap();

    let values: [[u8; 32]; 1] = trie.get(&[1u8; 32]).unwrap();
    assert_eq!(values[0], [1u8; 32]);

    // zkTrie is lazy, won't update the backend database until `commit` is called.
    assert!(trie.is_dirty());

    trie.commit().unwrap();let mut trie = ZkTrie::new(HashMapDb::new(), NoCacheHasher);
    // or this is default mode
    // let mut trie = ZkTrie::default();

    trie.raw_update(&[1u8; 32], vec![[1u8; 32]], 1).unwrap();

    let values: [[u8; 32]; 1] = trie.get(&[1u8; 32]).unwrap();
    assert_eq!(values[0], [1u8; 32]);

    // zkTrie is lazy, won't update the backend database until `commit` is called.
    assert!(trie.is_dirty());

    trie.commit().unwrap();
}
```

Check the [documentation](https://scroll-tech.github.io/zktrie-ng/zktrie_ng/index.html) for more details.