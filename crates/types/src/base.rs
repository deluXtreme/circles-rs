//! # Core Types
//!
//! - [`TransferStep`] - Single transfer operation in a multi-hop path
//! - [`FlowEdge`] - Directed edge in flow graph with routing info
//! - [`Stream`] - Collection of edges representing a transfer route
//! - [`FlowMatrix`] - Complete flow representation for contracts
//! - [`Address`] - Ethereum address (re-exported from alloy-primitives)
//! - [`Hex`] - Hexadecimal string type
//! - [`Hash`] - Transaction or block hash type
//! - [`ContractConfig`] - Generic contract configuration
//! - [`TransactionRequest`] - Transaction request object
//! - [`CallResult`] - Call result wrapper

use alloy_json_abi::JsonAbi;
use serde::{Deserialize, Serialize};

// =============================================================================
// Base EVM Types
// =============================================================================

/// Ethereum address type (re-exported from alloy-primitives)
pub type Address = alloy_primitives::Address;

/// Hexadecimal bytes data type (re-exported from alloy-primitives)
pub type Hex = alloy_primitives::Bytes;

/// Transaction hash type
pub type TxHash = alloy_primitives::aliases::TxHash;

/// Block hash type
pub type BlockHash = alloy_primitives::aliases::BlockHash;

/// Generic contract configuration
/// Contains address and ABI for a smart contract
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContractConfig {
    pub address: Address,
    pub abi: JsonAbi,
}

/// Transaction request object
/// Contains all data needed to send a transaction
pub type TransactionRequest = alloy_rpc_types::TransactionRequest;

/// Call result wrapper
/// Represents the result of a contract call
pub type TransportResult<T> = alloy_provider::transport::TransportResult<T>;
