use crate::{TokenBalance, TrustRelation};
use serde::{Deserialize, Serialize};

/// Event types for network events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    #[serde(rename = "CrcV1_Trust")]
    CrcV1Trust,
    #[serde(rename = "CrcV1_HubTransfer")]
    CrcV1HubTransfer,
    #[serde(rename = "CrcV1_Signup")]
    CrcV1Signup,
    #[serde(rename = "CrcV1_OrganizationSignup")]
    CrcV1OrganizationSignup,
    #[serde(rename = "CrcV2_RegisterHuman")]
    CrcV2RegisterHuman,
    #[serde(rename = "CrcV2_RegisterOrganization")]
    CrcV2RegisterOrganization,
    #[serde(rename = "CrcV2_RegisterGroup")]
    CrcV2RegisterGroup,
    #[serde(rename = "CrcV2_Trust")]
    CrcV2Trust,
    #[serde(rename = "CrcV2_TransferSingle")]
    CrcV2TransferSingle,
    #[serde(rename = "CrcV2_TransferBatch")]
    CrcV2TransferBatch,
}

/// Network snapshot structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSnapshot {
    pub trust_relations: Vec<TrustRelation>,
    pub balances: Vec<TokenBalance>,
    pub block_number: u64,
    pub timestamp: u64,
}
