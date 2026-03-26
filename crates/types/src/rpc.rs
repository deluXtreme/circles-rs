use crate::AvatarInfo;
use alloy_primitives::{Address, TxHash, U256};
use serde::{Deserialize, Deserializer, Serialize};

/// JSON-RPC request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest<T = serde_json::Value> {
    pub jsonrpc: String,
    pub id: serde_json::Value, // Can be number or string
    pub method: String,
    pub params: T,
}

impl<T> JsonRpcRequest<T> {
    pub fn new(id: impl Into<serde_json::Value>, method: String, params: T) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            method,
            params,
        }
    }
}

/// JSON-RPC error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// JSON-RPC response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse<T = serde_json::Value> {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl<T> JsonRpcResponse<T> {
    pub fn success(id: impl Into<serde_json::Value>, result: T) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: impl Into<serde_json::Value>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            result: None,
            error: Some(error),
        }
    }
}

/// Circles query response format
/// Used for circles_query RPC method results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CirclesQueryResponse {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

/// Generic query response wrapper
/// Used for internal query transformations and type-safe responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse<T = serde_json::Value> {
    pub result: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

impl<T> QueryResponse<T> {
    pub fn success(result: T) -> Self {
        Self {
            result,
            error: None,
        }
    }

    pub fn error(error: serde_json::Value) -> Self {
        Self {
            result: unsafe { std::mem::zeroed() }, // This is a hack, in practice we'd use Option<T>
            error: Some(error),
        }
    }
}

/// Better version of QueryResponse that's more idiomatic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeQueryResponse<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

impl<T> SafeQueryResponse<T> {
    pub fn success(result: T) -> Self {
        Self {
            result: Some(result),
            error: None,
        }
    }

    pub fn error(error: serde_json::Value) -> Self {
        Self {
            result: None,
            error: Some(error),
        }
    }
}

/// Unified invitation-origin response from `circles_getInvitationOrigin`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitationOriginResponse {
    pub address: Address,
    pub invitation_type: String,
    pub inviter: Option<Address>,
    pub proxy_inviter: Option<Address>,
    pub escrow_amount: Option<String>,
    pub block_number: u64,
    pub timestamp: u64,
    pub transaction_hash: TxHash,
    pub version: u32,
}

/// Trust-based invitation information returned by `circles_getAllInvitations`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustInvitation {
    pub address: Address,
    pub source: String,
    pub balance: String,
    pub avatar_info: Option<AvatarInfo>,
}

/// Escrow-based invitation information returned by `circles_getAllInvitations`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EscrowInvitation {
    pub address: Address,
    pub source: String,
    pub escrowed_amount: String,
    pub escrow_days: u32,
    pub block_number: u64,
    pub timestamp: u64,
    pub avatar_info: Option<AvatarInfo>,
}

/// At-scale invitation information returned by `circles_getAllInvitations`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtScaleInvitation {
    pub address: Address,
    pub source: String,
    pub block_number: u64,
    pub timestamp: u64,
    pub origin_inviter: Option<Address>,
}

/// Combined invitation response from `circles_getAllInvitations`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllInvitationsResponse {
    pub address: Address,
    pub trust_invitations: Vec<TrustInvitation>,
    pub escrow_invitations: Vec<EscrowInvitation>,
    pub at_scale_invitations: Vec<AtScaleInvitation>,
}

/// Account information returned by `circles_getInvitationsFrom`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitedAccountInfo {
    pub address: Address,
    pub status: String,
    pub block_number: u64,
    pub timestamp: u64,
    pub avatar_info: Option<AvatarInfo>,
}

/// Response returned by `circles_getInvitationsFrom`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitationsFromResponse {
    pub address: Address,
    pub accepted: bool,
    pub results: Vec<InvitedAccountInfo>,
}

/// Balance type that can be either raw U256 or formatted as TimeCircles floating point
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Balance {
    Raw(U256),
    TimeCircles(f64),
}

/// Token balance response from circles_getTokenBalances
#[derive(Debug, Clone, Serialize)]
pub struct TokenBalanceResponse {
    #[serde(rename = "tokenAddress")]
    pub token_address: Address,
    #[serde(rename = "tokenId")]
    pub token_id: Address,
    pub balance: Balance,
    /// Static atto-circles (inflationary wrappers) when provided by the backend.
    #[serde(default, rename = "staticAttoCircles")]
    pub static_atto_circles: Option<U256>,
    #[serde(default, rename = "staticCircles")]
    pub static_circles: Option<f64>,
    #[serde(default, rename = "tokenType", skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    #[serde(
        default,
        rename = "attoCircles",
        skip_serializing_if = "Option::is_none"
    )]
    pub atto_circles: Option<U256>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub circles: Option<f64>,
    #[serde(default, rename = "attoCrc", skip_serializing_if = "Option::is_none")]
    pub atto_crc: Option<U256>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crc: Option<f64>,
    #[serde(default, rename = "isErc20")]
    pub is_erc20: bool,
    #[serde(default, rename = "isErc1155")]
    pub is_erc1155: bool,
    #[serde(default, rename = "isWrapped")]
    pub is_wrapped: bool,
    #[serde(default, rename = "isInflationary")]
    pub is_inflationary: bool,
    #[serde(default, rename = "isGroup")]
    pub is_group: bool,
    #[serde(rename = "tokenOwner")]
    pub token_owner: Address,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenBalanceResponseWire {
    #[serde(default, rename = "tokenAddress", alias = "token_address")]
    token_address: Option<Address>,
    #[serde(rename = "tokenId", alias = "token_id")]
    token_id: Address,
    #[serde(default)]
    balance: Option<Balance>,
    #[serde(default, rename = "staticAttoCircles")]
    static_atto_circles: Option<U256>,
    #[serde(default, rename = "staticCircles")]
    static_circles: Option<f64>,
    #[serde(default, rename = "tokenType", alias = "token_type")]
    token_type: Option<String>,
    #[serde(default)]
    version: Option<u32>,
    #[serde(default, rename = "attoCircles")]
    atto_circles: Option<U256>,
    #[serde(default)]
    circles: Option<f64>,
    #[serde(default, rename = "attoCrc")]
    atto_crc: Option<U256>,
    #[serde(default)]
    crc: Option<f64>,
    #[serde(default, rename = "isErc20", alias = "is_erc20")]
    is_erc20: bool,
    #[serde(default, rename = "isErc1155", alias = "is_erc1155")]
    is_erc1155: bool,
    #[serde(default, rename = "isWrapped", alias = "is_wrapped")]
    is_wrapped: bool,
    #[serde(default, rename = "isInflationary", alias = "is_inflationary")]
    is_inflationary: bool,
    #[serde(default, rename = "isGroup", alias = "is_group")]
    is_group: bool,
    #[serde(rename = "tokenOwner", alias = "token_owner")]
    token_owner: Address,
}

impl<'de> Deserialize<'de> for TokenBalanceResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = TokenBalanceResponseWire::deserialize(deserializer)?;
        let balance = wire
            .balance
            .or_else(|| wire.atto_circles.map(Balance::Raw))
            .or_else(|| wire.circles.map(Balance::TimeCircles))
            .ok_or_else(|| serde::de::Error::missing_field("balance / attoCircles / circles"))?;

        Ok(Self {
            token_address: wire.token_address.unwrap_or(wire.token_id),
            token_id: wire.token_id,
            balance,
            static_atto_circles: wire.static_atto_circles,
            static_circles: wire.static_circles,
            token_type: wire.token_type,
            version: wire.version,
            atto_circles: wire.atto_circles,
            circles: wire.circles,
            atto_crc: wire.atto_crc,
            crc: wire.crc,
            is_erc20: wire.is_erc20,
            is_erc1155: wire.is_erc1155,
            is_wrapped: wire.is_wrapped,
            is_inflationary: wire.is_inflationary,
            is_group: wire.is_group,
            token_owner: wire.token_owner,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn invitation_origin_response_deserializes_plugin_shape() {
        let value = json!({
            "address": "0xde374ece6fa50e781e81aac78e811b33d16912c7",
            "invitationType": "v2_at_scale",
            "inviter": "0x1234567890abcdef1234567890abcdef12345678",
            "proxyInviter": "0xabcdef1234567890abcdef1234567890abcdef12",
            "escrowAmount": null,
            "blockNumber": 36500000,
            "timestamp": 1704240000,
            "transactionHash": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "version": 2
        });

        let response: InvitationOriginResponse =
            serde_json::from_value(value).expect("deserialize invitation origin");

        assert_eq!(response.invitation_type, "v2_at_scale");
        assert_eq!(response.version, 2);
        assert_eq!(response.block_number, 36_500_000);
        assert!(response.inviter.is_some());
        assert!(response.proxy_inviter.is_some());
    }

    #[test]
    fn all_invitations_response_deserializes_plugin_shape() {
        let value = json!({
            "address": "0xde374ece6fa50e781e81aac78e811b33d16912c7",
            "trustInvitations": [{
                "address": "0x1234567890abcdef1234567890abcdef12345678",
                "source": "trust",
                "balance": "150.5",
                "avatarInfo": null
            }],
            "escrowInvitations": [{
                "address": "0xabcdef1234567890abcdef1234567890abcdef12",
                "source": "escrow",
                "escrowedAmount": "100000000000000000000",
                "escrowDays": 7,
                "blockNumber": 43645581,
                "timestamp": 1765725505,
                "avatarInfo": null
            }],
            "atScaleInvitations": [{
                "address": "0xde374ece6fa50e781e81aac78e811b33d16912c7",
                "source": "atScale",
                "blockNumber": 43260668,
                "timestamp": 1763742205,
                "originInviter": null
            }]
        });

        let response: AllInvitationsResponse =
            serde_json::from_value(value).expect("deserialize all invitations");

        assert_eq!(response.trust_invitations.len(), 1);
        assert_eq!(response.escrow_invitations.len(), 1);
        assert_eq!(response.at_scale_invitations.len(), 1);
        assert_eq!(response.trust_invitations[0].balance, "150.5");
        assert_eq!(response.escrow_invitations[0].escrow_days, 7);
    }
}

/// Transaction history row matching the TS RPC helper shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionHistoryRow {
    pub block_number: u64,
    pub timestamp: u64,
    pub transaction_index: u32,
    pub log_index: u32,
    pub transaction_hash: TxHash,
    pub version: u32,
    pub from: Address,
    pub to: Address,
    pub id: String,
    pub token_address: Address,
    pub value: String,
    #[serde(default)]
    pub circles: Option<f64>,
    #[serde(default)]
    pub atto_circles: Option<U256>,
    #[serde(default)]
    pub static_circles: Option<f64>,
    #[serde(default)]
    pub static_atto_circles: Option<U256>,
    #[serde(default)]
    pub crc: Option<f64>,
    #[serde(default)]
    pub atto_crc: Option<U256>,
}
