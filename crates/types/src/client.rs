use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};

/// Avatar row data from RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarRow {
    pub address: Address,
    pub version: u32,
    #[serde(rename = "type")]
    pub avatar_type: String,
    /// Profile CID stored in the name registry
    pub cid_v0: Option<String>,
    // Additional fields as needed
}

/// Token balance row from RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalanceRow {
    pub token_address: Address,
    pub balance: U256,
    // Additional fields as needed
}

/// Trust relation row from RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustRelationRow {
    pub truster: Address,
    pub trustee: Address,
    pub expiry_time: u64,
}

/// Circles query result with pagination
/// Note: This is a trait-like interface in TypeScript, but we'll use a struct with a callback
#[derive(Debug, Clone)]
pub struct CirclesQuery<T> {
    pub rows: Vec<T>,
    pub has_more: bool,
    // In Rust, we'd typically use a function pointer or closure for the next_page functionality
    // This could be implemented as a method that takes a client reference
}

impl<T> CirclesQuery<T> {
    /// Create a new CirclesQuery
    pub fn new(rows: Vec<T>, has_more: bool) -> Self {
        Self { rows, has_more }
    }
}

/// Group type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupType {
    #[serde(rename = "Standard")]
    Standard,
    #[serde(rename = "Custom")]
    Custom,
}
