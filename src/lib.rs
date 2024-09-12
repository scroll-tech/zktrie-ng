//! # ZkTrie
//!
//! An rust implementation of zkTrie.
#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
extern crate tracing;

pub mod db;
// pub mod error;
pub mod hash;
pub mod trie;

// mod constants {
//     pub const HASHLEN: usize = 32;
//     pub const FIELDSIZE: usize = 32;
//     pub const ACCOUNTFIELDS: usize = 5;
//     pub const ACCOUNTSIZE: usize = FIELDSIZE * ACCOUNTFIELDS;
//     pub type Hash = [u8; HASHLEN];
//     pub type StoreData = [u8; FIELDSIZE];
//     pub type AccountData = [[u8; FIELDSIZE]; ACCOUNTFIELDS];
// }

// pub use constants::*;

#[cfg(feature = "hashbrown")]
pub(crate) use hashbrown::HashMap;
#[cfg(not(feature = "hashbrown"))]
pub(crate) use std::collections::HashMap;
