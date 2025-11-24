use alloy_primitives::{Address, TxHash};
use serde::{Deserialize, Serialize};

/// Group row information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupRow {
    pub block_number: u64,
    pub timestamp: u64,
    pub transaction_index: u32,
    pub log_index: u32,
    pub transaction_hash: TxHash,
    pub group: Address,
    #[serde(rename = "type")]
    pub group_type: String,
    pub owner: Address,
    pub mint_policy: Option<Address>,
    pub mint_handler: Option<Address>,
    pub treasury: Option<Address>,
    pub service: Option<Address>,
    pub fee_collection: Option<Address>,
    pub member_count: Option<u32>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub cid_v0_digest: Option<String>,
    pub erc20_wrapper_demurraged: Option<Address>,
    pub erc20_wrapper_static: Option<Address>,
}

/// Group membership row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMembershipRow {
    pub block_number: u64,
    pub timestamp: u64,
    pub transaction_index: u32,
    pub log_index: u32,
    pub transaction_hash: TxHash,
    pub group: Address,
    pub member: Address,
    pub expiry_time: u64,
}

/// Group query parameters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GroupQueryParams {
    pub name_starts_with: Option<String>,
    pub symbol_starts_with: Option<String>,
    pub group_address_in: Option<Vec<Address>>,
    pub group_type_in: Option<Vec<String>>,
    pub owner_in: Option<Vec<Address>>,
    pub mint_handler_equals: Option<Address>,
    pub treasury_equals: Option<Address>,
}
