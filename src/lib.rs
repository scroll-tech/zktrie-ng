//! # ZkTrie
//!
//! An rust implementation of zkTrie.
#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
extern crate tracing;
extern crate core;

pub mod db;
// pub mod error;
pub mod hash;
#[cfg(feature = "scroll")]
#[cfg_attr(docsrs, doc(cfg(feature = "scroll")))]
pub mod scroll_types;
pub mod trie;

#[cfg(feature = "hashbrown")]
pub(crate) use hashbrown::HashMap;
#[cfg(not(feature = "hashbrown"))]
pub(crate) use std::collections::HashMap;
