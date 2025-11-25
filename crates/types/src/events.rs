use alloy_primitives::TxHash;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Base event information shared by all Circles events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CirclesBaseEvent {
    pub block_number: u64,
    pub timestamp: Option<u64>,
    pub transaction_index: u32,
    pub log_index: u32,
    pub transaction_hash: Option<TxHash>,
}

/// All possible Circles event types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CirclesEventType {
    // HubV2 events
    #[serde(rename = "CrcV2_ApprovalForAll")]
    CrcV2ApprovalForAll,
    #[serde(rename = "CrcV2_DiscountCost")]
    CrcV2DiscountCost,
    #[serde(rename = "CrcV2_FlowEdgesScopeLastEnded")]
    CrcV2FlowEdgesScopeLastEnded,
    #[serde(rename = "CrcV2_FlowEdgesScopeSingleStarted")]
    CrcV2FlowEdgesScopeSingleStarted,
    #[serde(rename = "CrcV2_GroupMint")]
    CrcV2GroupMint,
    #[serde(rename = "CrcV2_PersonalMint")]
    CrcV2PersonalMint,
    #[serde(rename = "CrcV2_RegisterGroup")]
    CrcV2RegisterGroup,
    #[serde(rename = "CrcV2_RegisterHuman")]
    CrcV2RegisterHuman,
    #[serde(rename = "CrcV2_RegisterOrganization")]
    CrcV2RegisterOrganization,
    #[serde(rename = "CrcV2_SetAdvancedUsageFlag")]
    CrcV2SetAdvancedUsageFlag,
    #[serde(rename = "CrcV2_Stopped")]
    CrcV2Stopped,
    #[serde(rename = "CrcV2_StreamCompleted")]
    CrcV2StreamCompleted,
    #[serde(rename = "CrcV2_TransferBatch")]
    CrcV2TransferBatch,
    #[serde(rename = "CrcV2_TransferSingle")]
    CrcV2TransferSingle,
    #[serde(rename = "CrcV2_Trust")]
    CrcV2Trust,
    #[serde(rename = "CrcV2_URI")]
    CrcV2URI,

    // ERC20 Wrapper events
    #[serde(rename = "CrcV2_Approval")]
    CrcV2Approval,
    #[serde(rename = "CrcV2_DepositDemurraged")]
    CrcV2DepositDemurraged,
    #[serde(rename = "CrcV2_DepositInflationary")]
    CrcV2DepositInflationary,
    #[serde(rename = "CrcV2_EIP712DomainChanged")]
    CrcV2EIP712DomainChanged,
    #[serde(rename = "CrcV2_Transfer")]
    CrcV2Transfer,
    #[serde(rename = "CrcV2_WithdrawDemurraged")]
    CrcV2WithdrawDemurraged,
    #[serde(rename = "CrcV2_WithdrawInflationary")]
    CrcV2WithdrawInflationary,

    // Name Registry events
    #[serde(rename = "CrcV2_CidV0")]
    CrcV2CidV0,
    #[serde(rename = "CrcV2_RegisterShortName")]
    CrcV2RegisterShortName,
    #[serde(rename = "CrcV2_UpdateMetadataDigest")]
    CrcV2UpdateMetadataDigest,

    // Base Group events
    #[serde(rename = "CrcV2_GroupRedeemCollateralBurn")]
    CrcV2GroupRedeemCollateralBurn,
    #[serde(rename = "CrcV2_GroupRedeemCollateralReturn")]
    CrcV2GroupRedeemCollateralReturn,

    // Invitation events
    #[serde(rename = "CrcV2_InviteHuman")]
    CrcV2InviteHuman,

    // Unknown event fallback
    #[serde(rename = "Crc_UnknownEvent")]
    CrcUnknownEvent,
}

/// Generic Circles event with dynamic data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CirclesEvent {
    #[serde(flatten)]
    pub base: CirclesBaseEvent,

    #[serde(rename = "$event")]
    pub event_type: CirclesEventType,

    #[serde(flatten)]
    pub data: HashMap<String, serde_json::Value>,
}

/// RPC subscription event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcSubscriptionEvent {
    pub event: String,
    pub values: HashMap<String, serde_json::Value>,
}
