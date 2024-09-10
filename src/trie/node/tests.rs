use zktrie::HashField;
use zktrie_rust::hash::AsHash;
use zktrie_rust::types::Hashable;
use crate::hash::Poseidon;
use super::*;

type OldNode = zktrie_rust::types::Node::<AsHash<HashField>>;

#[test]
fn test_empty_node() {
    let expected = OldNode::new_empty_node();
    let node_hash = expected.calc_node_hash().unwrap().node_hash().unwrap();

    assert_eq!(Node::<Poseidon>::EMPTY.node_hash, node_hash.as_ref());
}

#[test]
fn test_leaf_node() {
    let expected = OldNode::new_leaf_node(
        AsHash::from_bytes(&[1u8; 32]).unwrap(),
        0,
        vec![[2u8; 32]],
    );
    let node_hash = expected.calc_node_hash().unwrap().node_hash().unwrap();

    let node = Node::<Poseidon>::new_leaf(
        Poseidon::new_hash_try_from_bytes(&[1u8; 32]).unwrap(),
        vec![[2u8; 32]],
        0,
        None,
    ).unwrap();

    assert_eq!(node.node_hash, node_hash.as_ref());
}

#[test]
fn test_branch_node() {
    let expected = OldNode::new_parent_node(
        zktrie_rust::types::NodeType::NodeTypeBranch0,
        AsHash::from_bytes(&[1u8; 32]).unwrap(),
        AsHash::from_bytes(&[2u8; 32]).unwrap(),
    );
    let node_hash = expected.calc_node_hash().unwrap().node_hash().unwrap();

    let node = Node::<Poseidon>::new_branch(
        BranchLTRT,
        Poseidon::new_hash_try_from_bytes(&[1u8; 32]).unwrap(),
        Poseidon::new_hash_try_from_bytes(&[2u8; 32]).unwrap(),
    ).unwrap();

    assert_eq!(node.node_hash, node_hash.as_ref());
}