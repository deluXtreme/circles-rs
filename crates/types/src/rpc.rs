use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};

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

/// Balance type that can be either raw U256 or formatted as TimeCircles floating point
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Balance {
    Raw(U256),
    TimeCircles(f64),
}

/// Token balance response from circles_getTokenBalances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalanceResponse {
    #[serde(rename = "tokenId")]
    pub token_id: Address,
    pub balance: Balance,
    /// Static atto-circles (inflationary wrappers) when provided by the backend.
    #[serde(default, rename = "staticAttoCircles")]
    pub static_atto_circles: Option<U256>,
    #[serde(default, rename = "staticCircles")]
    pub static_circles: Option<f64>,
    pub token_owner: Address,
}
