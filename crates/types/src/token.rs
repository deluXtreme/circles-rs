use alloy_primitives::{Address, TxHash, U256};
use serde::{Deserialize, Serialize};

/// Token balance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub token_address: Address,
    pub token_id: U256,
    pub token_owner: Address,
    pub token_type: String,
    pub version: u32,
    pub atto_circles: U256,
    pub circles: f64,
    pub static_atto_circles: U256,
    pub static_circles: f64,
    pub atto_crc: U256,
    pub crc: f64,
    pub is_erc20: bool,
    pub is_erc1155: bool,
    pub is_wrapped: bool,
    pub is_inflationary: bool,
    pub is_group: bool,
}

/// Token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub block_number: u64,
    pub timestamp: u64,
    pub transaction_index: u32,
    pub log_index: u32,
    pub transaction_hash: TxHash,
    pub version: u32,
    #[serde(rename = "type")]
    pub info_type: Option<String>,
    pub token_type: String,
    pub token: Address,
    pub token_owner: Address,
}

/// Token holder information from V_CrcV2_BalancesByAccountAndToken
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenHolder {
    pub account: Address,
    pub token_address: Address,
    pub demurraged_total_balance: String,
}
