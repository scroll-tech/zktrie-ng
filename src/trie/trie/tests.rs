use super::*;
use crate::hash::poseidon::tests::gen_random_bytes;
use crate::hash::poseidon::TRIE_MAX_LEVELS;
use rand::random;
use zktrie::HashField;
use zktrie_rust::db::SimpleDb;
use zktrie_rust::hash::AsHash;
use zktrie_rust::types::TrieHashScheme;

impl<const MAX_LEVEL: usize, H: HashScheme, Db: KVDatabase, CacheDb: KVDatabase>
    ZkTrie<MAX_LEVEL, H, Db, CacheDb>
{
    fn print_node(
        &self,
        node_hash: LazyNodeHash,
        level: usize,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let node = self.get_node(node_hash).unwrap();

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

impl<const MAX_LEVEL: usize, H: HashScheme, Db: KVDatabase, CacheDb: KVDatabase> Display
    for ZkTrie<MAX_LEVEL, H, Db, CacheDb>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.print_node(self.root.clone(), 0, f)
    }
}

type NodeOld = zktrie_rust::types::Node<AsHash<HashField>>;
type TrieOld = zktrie_rust::raw::ZkTrieImpl<AsHash<HashField>, SimpleDb, TRIE_MAX_LEVELS>;
fn new_trie_old() -> TrieOld {
    TrieOld::new_zktrie_impl(SimpleDb::new()).unwrap()
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

#[test]
fn test_simple() {
    let mut old_trie = new_trie_old();

    let mut trie = ZkTrie::<TRIE_MAX_LEVELS>::new(HashMapDb::new(), HashMapDb::new());

    let k = [1u8; 32];
    let v = vec![[1u8; 32], [2u8; 32], [3u8; 32]];

    let old_key = NodeOld::hash_bytes(&k).unwrap();
    old_trie.try_update(&old_key, 1, v.clone()).unwrap();
    old_trie.prepare_root().unwrap();

    trie.update(&k, v.clone(), 1).unwrap();
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

    let mut trie = ZkTrie::<TRIE_MAX_LEVELS>::new(HashMapDb::new(), HashMapDb::new());

    for _ in 0..50 {
        let k: [u8; 32] = random();

        let (values, compression_flag) = gen_random_bytes();
        let old_key = NodeOld::hash_bytes(&k).unwrap();
        old_trie
            .try_update(&old_key, compression_flag, values.clone())
            .unwrap();

        trie.update(&k, values, compression_flag).unwrap();
    }

    old_trie.prepare_root().unwrap();

    trie.commit().unwrap();

    assert_eq!(old_trie.root().as_ref(), trie.root.unwrap_ref().as_slice());
}
