use alloy_primitives::{Address, TxHash};
use serde::{Deserialize, Serialize};

/// Trust relation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustRelation {
    pub block_number: u64,
    pub timestamp: u64,
    pub transaction_index: u32,
    pub log_index: u32,
    pub transaction_hash: TxHash,
    pub truster: Address,
    pub trustee: Address,
    pub expiry_time: u64,
}

/// Trust relation type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustRelationType {
    #[serde(rename = "trusts")]
    Trusts,
    #[serde(rename = "trustedBy")]
    TrustedBy,
    #[serde(rename = "mutuallyTrusts")]
    MutuallyTrusts,
}

/// Aggregated trust relation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedTrustRelation {
    pub subject_avatar: Address,
    pub relation: TrustRelationType,
    pub object_avatar: Address,
    pub timestamp: u64,
}
