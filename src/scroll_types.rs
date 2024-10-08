//! Types Scroll used in zkTrie.
//!
//! # Example
//!
//! ```rust
//! use alloy_primitives::{address, B256};
//! use poseidon_bn254::{Fr, Field};
//! use rand::thread_rng;
//! use revm_primitives::AccountInfo;
//! use zktrie_ng::{hash::HashOutput, scroll_types::Account, trie::ZkTrie, db::NodeDb};
//!
//! let trie_db = NodeDb::default();
//! let mut trie = ZkTrie::default();
//!
//! let address = address!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
//! let account = AccountInfo::default();
//! let storage_root = Fr::random(thread_rng()).as_canonical_repr();
//!
//! let trie_account = Account::from_revm_account_with_storage_root(account, storage_root);
//!
//! trie.update(&trie_db, address, trie_account).unwrap();
//!
//! let account: Account = trie.get(&trie_db, address).unwrap().unwrap();
//!
//! assert_eq!(trie_account, account);
//! ```
use crate::hash::ZkHash;
use crate::trie::{DecodeValueBytes, EncodeValueBytes};
use alloy_primitives::{B256, U256};
use revm_primitives::AccountInfo;

/// Account data stored in zkTrie.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Account {
    /// nonce
    pub nonce: u64,
    /// code size
    pub code_size: u64,
    /// balance
    pub balance: U256,
    /// storage root
    pub storage_root: ZkHash,
    /// keccak code hash
    pub code_hash: B256,
    /// poseidon code hash
    pub poseidon_code_hash: B256,
}

impl EncodeValueBytes for &Account {
    fn encode_values_bytes(&self) -> (Vec<[u8; 32]>, u32) {
        (
            vec![
                U256::from_limbs([self.nonce, self.code_size, 0, 0]).to_be_bytes(),
                self.balance.to_be_bytes(),
                self.storage_root.0,
                self.code_hash.0,
                self.poseidon_code_hash.0,
            ],
            8,
        )
    }
}

impl EncodeValueBytes for Account {
    fn encode_values_bytes(&self) -> (Vec<[u8; 32]>, u32) {
        (&self).encode_values_bytes()
    }
}

impl DecodeValueBytes for Account {
    fn decode_values_bytes(values: &[[u8; 32]]) -> Option<Self> {
        let values: &[[u8; 32]; 5] = values.try_into().ok()?;
        Some(Account {
            nonce: u64::from_be_bytes(values[0][24..].try_into().unwrap()),
            code_size: u64::from_be_bytes(values[0][16..24].try_into().unwrap()),
            balance: U256::from_be_bytes(values[1]),
            storage_root: B256::from(values[2]),
            code_hash: B256::from(values[3]),
            poseidon_code_hash: B256::from(values[4]),
        })
    }
}

impl Account {
    /// Create an account from revm account and storage root.
    pub fn from_revm_account_with_storage_root(acc: AccountInfo, storage_root: B256) -> Self {
        Account {
            balance: acc.balance,
            nonce: acc.nonce,
            code_size: acc.code_size as u64,
            storage_root,
            code_hash: acc.code_hash,
            poseidon_code_hash: acc.poseidon_code_hash,
        }
    }
}

impl From<Account> for AccountInfo {
    fn from(acc: Account) -> Self {
        AccountInfo {
            balance: acc.balance,
            nonce: acc.nonce,
            code_size: acc.code_size as usize,
            code_hash: acc.code_hash,
            poseidon_code_hash: acc.poseidon_code_hash,
            code: None,
        }
    }
}

impl EncodeValueBytes for &U256 {
    fn encode_values_bytes(&self) -> (Vec<[u8; 32]>, u32) {
        (vec![self.to_be_bytes()], 1)
    }
}

impl EncodeValueBytes for U256 {
    fn encode_values_bytes(&self) -> (Vec<[u8; 32]>, u32) {
        (&self).encode_values_bytes()
    }
}

impl DecodeValueBytes for U256 {
    fn decode_values_bytes(values: &[[u8; 32]]) -> Option<Self> {
        values.first().map(|v| U256::from_be_bytes(*v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NodeDb;
    use crate::hash::HashOutput;
    use crate::trie::ZkTrie;
    use alloy_primitives::address;
    use poseidon_bn254::{Field, Fr};
    use rand::thread_rng;
    use revm_primitives::AccountInfo;

    #[test]
    fn test_account() {
        let trie_db = NodeDb::default();
        let mut trie = ZkTrie::default();

        let address = address!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
        let account = AccountInfo::default();
        let storage_root = Fr::random(thread_rng()).as_canonical_repr();

        let trie_account = Account::from_revm_account_with_storage_root(account, storage_root);

        trie.update(&trie_db, address, trie_account).unwrap();

        let account = trie
            .get::<_, Account, _>(&trie_db, address)
            .unwrap()
            .unwrap();

        assert_eq!(trie_account, account);
    }
}
