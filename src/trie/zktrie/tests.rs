use super::*;
use crate::hash::poseidon::tests::gen_random_bytes;
use rand::random;
use rand::seq::SliceRandom;
use std::fmt::Display;
use zktrie::HashField;
use zktrie_rust::{db::SimpleDb, hash::AsHash, types::TrieHashScheme};

type NodeOld = zktrie_rust::types::Node<AsHash<HashField>>;
type TrieOld =
    zktrie_rust::raw::ZkTrieImpl<AsHash<HashField>, SimpleDb, { Poseidon::TRIE_MAX_LEVELS }>;
fn new_trie_old() -> TrieOld {
    TrieOld::new_zktrie_impl(SimpleDb::new()).unwrap()
}

#[test]
fn test_simple() {
    let mut old_trie = new_trie_old();

    let mut trie = ZkTrie::default();

    let k = [1u8; 32];
    let v = vec![[1u8; 32], [2u8; 32], [3u8; 32]];

    let old_key = NodeOld::hash_bytes(&k).unwrap();
    old_trie.try_update(&old_key, 1, v.clone()).unwrap();
    old_trie.prepare_root().unwrap();

    trie.raw_update(&k, v.clone(), 1).unwrap();
    trie.commit().unwrap();

    assert_eq!(old_trie.root().as_ref(), trie.root.unwrap_ref().as_slice());
}

#[test]
fn test_randoms() {
    for _ in 0..10 {
        test_random();
    }
}

fn test_random() {
    let mut old_trie = new_trie_old();

    let mut trie = ZkTrie::default();

    let mut keys = Vec::new();

    for _ in 0..2 {
        for _ in 0..10 {
            let k: [u8; 32] = random();

            let (values, compression_flag) = gen_random_bytes();
            let old_key = NodeOld::hash_bytes(&k).unwrap();
            old_trie
                .try_update(&old_key, compression_flag, values.clone())
                .unwrap();

            trie.raw_update(&k, values, compression_flag).unwrap();

            keys.push((k, old_key));
        }

        old_trie.prepare_root().unwrap();
        old_trie.commit().unwrap();
        trie.commit().unwrap();
    }

    trie.full_gc().unwrap();

    for (k, _) in keys.iter() {
        let node_key = <NoCacheHasher as KeyHasher<Poseidon>>::hash(&NoCacheHasher, k).unwrap();
        // full gc didn't delete anything unexpected
        trie.get_node_by_key(&node_key).unwrap();
    }

    assert_eq!(old_trie.root().as_ref(), trie.root.unwrap_ref().as_slice());

    for (k, old_key) in keys.choose_multiple(&mut rand::thread_rng(), 10) {
        old_trie.try_delete(old_key).unwrap();
        trie.delete(k).unwrap();
    }

    old_trie.prepare_root().unwrap();
    old_trie.commit().unwrap();
    trie.commit().unwrap();

    // println!("Old:");
    // print_old_trie(&old_trie, old_trie.root().clone(), 0);
    // println!("New:");
    // println!("{}", trie);

    trie.full_gc().unwrap();

    assert_eq!(old_trie.root().as_ref(), trie.root.unwrap_ref().as_slice());
}

#[allow(dead_code)]
fn print_old_trie(trie: &TrieOld, hash: AsHash<HashField>, level: usize) {
    use zktrie_rust::types::NodeType::*;
    let node = trie.get_node(&hash).unwrap().calc_node_hash().unwrap();

    let lead_char = if level == 0 { "" } else { "├ " };

    match node.node_type {
        NodeTypeEmptyNew => {
            println!("{}{lead_char}Empty", "  ".repeat(level));
        }
        NodeTypeLeafNew => {
            println!(
                "{}{lead_char}Leaf: {:?}",
                "  ".repeat(level),
                node.node_hash().unwrap().as_ref()
            );
        }
        _ => {
            println!(
                "{}{lead_char}Branch({:?}): {:?}",
                "  ".repeat(level),
                node.node_type,
                hash.as_ref()
            );
            print_old_trie(trie, node.child_left.unwrap(), level + 1);
            print_old_trie(trie, node.child_right.unwrap(), level + 1);
        }
    };
}

impl<H: HashScheme, Db: KVDatabase, K: KeyHasher<H>> ZkTrie<H, Db, K> {
    fn print_node(
        &self,
        node_hash: LazyNodeHash,
        level: usize,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let node = self.get_node_by_hash(node_hash).unwrap();

        let lead_char = if level == 0 { "" } else { "├ " };

        match node.node_type() {
            NodeType::Empty => {
                writeln!(f, "{}{lead_char}Empty", "  ".repeat(level))?;
            }
            NodeType::Leaf => {
                let leaf = unsafe { node.get_node_hash_unchecked() };
                writeln!(
                    f,
                    "{}{lead_char}Leaf: {:?}",
                    "  ".repeat(level),
                    leaf.as_slice()
                )?;
            }
            _ => {
                let branch = node.as_branch().unwrap();
                let hash = node.get_or_calculate_node_hash().unwrap();
                writeln!(
                    f,
                    "{}{lead_char}Branch({:?}): {:?}",
                    "  ".repeat(level),
                    branch.node_type(),
                    hash.as_slice()
                )?;
                self.print_node(branch.child_left().clone(), level + 1, f)?;
                self.print_node(branch.child_right().clone(), level + 1, f)?;
            }
        };

        Ok(())
    }
}

impl<H: HashScheme, Db: KVDatabase, K: KeyHasher<H>> Display for ZkTrie<H, Db, K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.print_node(self.root.clone(), 0, f)
    }
}
