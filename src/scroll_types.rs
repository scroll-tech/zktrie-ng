//! Types Scroll used in zkTrie.
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
    pub storage_root: B256,
    /// keccak code hash
    pub code_hash: B256,
    /// poseidon code hash
    pub poseidon_code_hash: B256,
}

impl EncodeValueBytes for Account {
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

impl DecodeValueBytes<5> for Account {
    fn decode_values_bytes(values: &[[u8; 32]; 5]) -> Self {
        Account {
            nonce: u64::from_be_bytes(values[0][24..].try_into().unwrap()),
            code_size: u64::from_be_bytes(values[0][16..24].try_into().unwrap()),
            balance: U256::from_be_bytes(values[1]),
            storage_root: B256::from(values[2]),
            code_hash: B256::from(values[3]),
            poseidon_code_hash: B256::from(values[4]),
        }
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

impl EncodeValueBytes for U256 {
    fn encode_values_bytes(&self) -> (Vec<[u8; 32]>, u32) {
        (vec![self.to_be_bytes()], 1)
    }
}

impl DecodeValueBytes<1> for U256 {
    fn decode_values_bytes(values: &[[u8; 32]; 1]) -> Self {
        U256::from_be_bytes(values[0])
    }
}
