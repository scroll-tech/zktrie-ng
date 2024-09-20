//! # ZkTrie
//!
//! An rust implementation of zkTrie.
//!
//! ## Example
//!
//! ### In memory zkTrie using Poseidon hash
//!
//! ```rust
//! use zktrie_ng::{
//!     trie,
//!     hash::{
//!         key_hasher::NoCacheHasher,
//!         poseidon::Poseidon,
//!     },
//!     db::HashMapDb,
//! };
//!
//! // A ZkTrie using Poseidon hash scheme,
//! // HashMap as backend kv database and NoCacheHasher as key hasher.
//! type ZkTrie = trie::ZkTrie<Poseidon, HashMapDb, NoCacheHasher>;
//!
//! let mut trie = ZkTrie::new(HashMapDb::default(), NoCacheHasher);
//! // or this is default mode
//! // let mut trie = ZkTrie::default();
//!
//! trie.raw_update(&[1u8; 32], vec![[1u8; 32]], 1).unwrap();
//!
//! let values: [[u8; 32]; 1] = trie.get(&[1u8; 32]).unwrap().unwrap();
//! assert_eq!(values[0], [1u8; 32]);
//!
//! // zkTrie is lazy, won't update the backend database until `commit` is called.
//! assert!(trie.is_dirty());
//!
//! trie.commit().unwrap();
//! ```
//!
//! ### On disk zkTrie using Poseidon hash
//!
//! See [`db::sled`] for more information.
//!
#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
extern crate tracing;
extern crate core;

pub mod db;
pub mod hash;
#[cfg(feature = "scroll")]
#[cfg_attr(docsrs, doc(cfg(feature = "scroll")))]
pub mod scroll_types;
pub mod trie;

#[cfg(feature = "hashbrown")]
pub(crate) use hashbrown::{HashMap, HashSet};
#[cfg(not(feature = "hashbrown"))]
pub(crate) use std::collections::{HashMap, HashSet};

#[cfg(test)]
#[ctor::ctor]
fn setup_tracing() {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("trace")),
        )
        .init();
}
