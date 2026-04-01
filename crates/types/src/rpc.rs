use crate::{AvatarInfo, Profile};
use alloy_primitives::{Address, TxHash, U256};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::str::FromStr;

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

/// Generic cursor-based page returned by dedicated RPC helper methods.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedResponse<TRow> {
    pub results: Vec<TRow>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

/// Consolidated profile view returned by `circles_getProfileView`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileView {
    pub address: Address,
    pub avatar_info: Option<AvatarInfo>,
    pub profile: Option<Profile>,
    pub trust_stats: TrustStats,
    pub v1_balance: Option<String>,
    pub v2_balance: Option<String>,
}

/// Trust counters embedded in `ProfileView`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustStats {
    pub trusts_count: u32,
    pub trusted_by_count: u32,
}

/// Trust-network summary returned by `circles_getTrustNetworkSummary`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustNetworkSummary {
    pub address: Address,
    pub direct_trusts_count: u32,
    pub direct_trusted_by_count: u32,
    pub mutual_trusts_count: u32,
    pub mutual_trusts: Vec<Address>,
    pub network_reach: u32,
}

/// Enriched trust-relation row returned by `circles_getAggregatedTrustRelationsEnriched`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustRelationInfo {
    pub address: Address,
    pub avatar_info: Option<AvatarInfo>,
    pub relation_type: String,
}

/// Counts grouped by relation type for enriched trust queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustRelationCounts {
    pub mutual: u32,
    pub trusts: u32,
    pub trusted_by: u32,
    pub total: u32,
}

/// Paginated enriched trust relations returned by `circles_getAggregatedTrustRelationsEnriched`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedAggregatedTrustRelationsResponse {
    pub address: Address,
    pub results: Vec<TrustRelationInfo>,
    pub counts: TrustRelationCounts,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

/// Inviter row returned by `circles_getValidInviters`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InviterInfo {
    pub address: Address,
    pub balance: String,
    pub avatar_info: Option<AvatarInfo>,
}

/// Paginated valid-inviter response returned by `circles_getValidInviters`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedValidInvitersResponse {
    pub address: Address,
    pub results: Vec<InviterInfo>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

/// Participant profile/avatar data embedded in an enriched transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantInfo {
    pub avatar_info: Option<AvatarInfo>,
    pub profile: Option<Profile>,
}

/// Enriched transaction row returned by `circles_getTransactionHistoryEnriched`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichedTransaction {
    pub block_number: u64,
    pub timestamp: u64,
    pub transaction_hash: TxHash,
    pub transaction_index: u32,
    pub log_index: u32,
    pub event: serde_json::Value,
    pub participants: BTreeMap<String, ParticipantInfo>,
}

/// Paginated unified profile-search response returned by `circles_searchProfileByAddressOrName`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedProfileSearchResponse {
    pub query: String,
    pub search_type: String,
    pub results: Vec<Profile>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

/// Optional parameters for `circles_getTransactionHistoryEnriched`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichedTransactionHistoryOptions {
    pub to_block: Option<u64>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
    pub version: Option<u32>,
    pub exclude_intermediary: Option<bool>,
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

    #[test]
    fn profile_view_deserializes_plugin_shape() {
        let value = json!({
            "address": "0xde374ece6fa50e781e81aac78e811b33d16912c7",
            "avatarInfo": null,
            "profile": {
                "name": "franco",
                "description": "builder"
            },
            "trustStats": {
                "trustsCount": 4,
                "trustedByCount": 7
            },
            "v1Balance": null,
            "v2Balance": "123.45"
        });

        let response: ProfileView =
            serde_json::from_value(value).expect("deserialize profile view");

        assert_eq!(response.trust_stats.trusts_count, 4);
        assert_eq!(response.trust_stats.trusted_by_count, 7);
        assert_eq!(response.v2_balance.as_deref(), Some("123.45"));
        assert_eq!(
            response.profile.as_ref().map(|p| p.name.as_str()),
            Some("franco")
        );
    }

    #[test]
    fn paged_aggregated_trust_relations_deserialize_plugin_shape() {
        let value = json!({
            "address": "0xde374ece6fa50e781e81aac78e811b33d16912c7",
            "results": [{
                "address": "0x1234567890abcdef1234567890abcdef12345678",
                "avatarInfo": null,
                "relationType": "mutual"
            }],
            "counts": {
                "mutual": 1,
                "trusts": 2,
                "trustedBy": 3,
                "total": 6
            },
            "hasMore": true,
            "nextCursor": "Zm9v"
        });

        let response: PagedAggregatedTrustRelationsResponse =
            serde_json::from_value(value).expect("deserialize paged trust relations");

        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].relation_type, "mutual");
        assert_eq!(response.counts.total, 6);
        assert!(response.has_more);
        assert_eq!(response.next_cursor.as_deref(), Some("Zm9v"));
    }

    #[test]
    fn enriched_transaction_page_deserializes_nested_shape() {
        let value = json!({
            "results": [{
                "blockNumber": 123,
                "timestamp": 456,
                "transactionHash": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "transactionIndex": 1,
                "logIndex": 2,
                "event": {
                    "from": "0xde374ece6fa50e781e81aac78e811b33d16912c7",
                    "to": "0x1234567890abcdef1234567890abcdef12345678",
                    "value": "100"
                },
                "participants": {
                    "0xde374ece6fa50e781e81aac78e811b33d16912c7": {
                        "avatarInfo": null,
                        "profile": {
                            "name": "sender"
                        }
                    }
                }
            }],
            "hasMore": false,
            "nextCursor": null
        });

        let response: PagedResponse<EnrichedTransaction> =
            serde_json::from_value(value).expect("deserialize enriched tx page");

        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].block_number, 123);
        assert!(response.results[0]
            .participants
            .contains_key("0xde374ece6fa50e781e81aac78e811b33d16912c7"));
        assert!(!response.has_more);
    }

    #[test]
    fn paged_profile_search_response_deserializes_plugin_shape() {
        let value = json!({
            "query": "berlin",
            "searchType": "text",
            "results": [{
                "name": "Berlin CRC",
                "description": "community"
            }],
            "hasMore": true,
            "nextCursor": "YmFy"
        });

        let response: PagedProfileSearchResponse =
            serde_json::from_value(value).expect("deserialize profile search response");

        assert_eq!(response.search_type, "text");
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].name, "Berlin CRC");
        assert!(response.has_more);
    }
}

fn deserialize_option_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<serde_json::Value>::deserialize(deserializer)? {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(number)) => number
            .as_f64()
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom("invalid f64 number")),
        Some(serde_json::Value::String(raw)) => raw
            .parse::<f64>()
            .map(Some)
            .map_err(|e| serde::de::Error::custom(e.to_string())),
        Some(other) => Err(serde::de::Error::custom(format!(
            "expected string or number, got {other}"
        ))),
    }
}

fn deserialize_option_u256<'de, D>(deserializer: D) -> Result<Option<U256>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<serde_json::Value>::deserialize(deserializer)? {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(number)) => U256::from_str(&number.to_string())
            .map(Some)
            .map_err(|e| serde::de::Error::custom(e.to_string())),
        Some(serde_json::Value::String(raw)) => U256::from_str(&raw)
            .map(Some)
            .map_err(|e| serde::de::Error::custom(e.to_string())),
        Some(other) => Err(serde::de::Error::custom(format!(
            "expected string or number, got {other}"
        ))),
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
    #[serde(default)]
    pub operator: Option<Address>,
    pub from: Address,
    pub to: Address,
    #[serde(default)]
    pub id: Option<String>,
    pub token_address: Address,
    pub value: String,
    #[serde(default, deserialize_with = "deserialize_option_f64")]
    pub circles: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_option_u256")]
    pub atto_circles: Option<U256>,
    #[serde(default, deserialize_with = "deserialize_option_f64")]
    pub static_circles: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_option_u256")]
    pub static_atto_circles: Option<U256>,
    #[serde(default, deserialize_with = "deserialize_option_f64")]
    pub crc: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_option_u256")]
    pub atto_crc: Option<U256>,
}

#[cfg(test)]
mod transaction_history_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn transaction_history_row_deserializes_native_rpc_shape() {
        let value = json!({
            "blockNumber": 123,
            "timestamp": 456,
            "transactionIndex": 1,
            "logIndex": 2,
            "transactionHash": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "version": 2,
            "operator": "0x1111111111111111111111111111111111111111",
            "from": "0x2222222222222222222222222222222222222222",
            "to": "0x3333333333333333333333333333333333333333",
            "id": null,
            "tokenAddress": "0x4444444444444444444444444444444444444444",
            "value": "100",
            "circles": "0.1",
            "attoCircles": "100",
            "crc": "0.2",
            "attoCrc": "200",
            "staticCircles": "0.3",
            "staticAttoCircles": "300"
        });

        let row: TransactionHistoryRow =
            serde_json::from_value(value).expect("deserialize native tx history row");

        assert_eq!(row.block_number, 123);
        assert_eq!(row.operator, Some(Address::repeat_byte(0x11)));
        assert_eq!(row.id, None);
        assert_eq!(row.circles, Some(0.1));
        assert_eq!(row.atto_circles, Some(U256::from(100)));
        assert_eq!(row.static_atto_circles, Some(U256::from(300)));
    }
}
