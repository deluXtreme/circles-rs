//! # Circles Pathfinder
//!
//! Pathfinding and flow matrix calculation for the Circles protocol.
//!
//! This crate provides efficient pathfinding algorithms and flow matrix generation
//! for the Circles universal basic income protocol, with ready-to-use types for
//! smart contract interactions.
//!
//! ## Quick Start
//!
//! ### High-level API (Recommended)
//!
//! ```rust,no_run
//! use circles_pathfinder::{FindPathParams, prepare_flow_for_contract};
//! use alloy_primitives::{Address, aliases::U192};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let params = FindPathParams {
//!     from: "0x123...".parse()?,
//!     to: "0x456...".parse()?,
//!     target_flow: U192::from(1_000_000_000_000_000_000u64), // 1 ETH
//!     use_wrapped_balances: Some(true),
//!     from_tokens: None,
//!     to_tokens: None,
//!     exclude_from_tokens: None,
//!     exclude_to_tokens: None,
//! };
//!
//! // One function call gets contract-ready data
//! let matrix = prepare_flow_for_contract("https://rpc.circles.com", params).await?;
//!
//! // Ready for smart contract calls
//! // contract.some_function(matrix.flow_vertices, matrix.flow_edges, ...)
//! # Ok(())
//! # }
//! ```
//!
//! ### Low-level API (Advanced)
//!
//! ```rust,no_run
//! use circles_pathfinder::{find_path, create_flow_matrix};
//! use alloy_primitives::{Address, aliases::U192};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Step 1: Find path
//! let transfers = find_path(
//!     "https://rpc.circles.com",
//!     "0x123...".parse()?,
//!     "0x456...".parse()?,
//!     U192::from(1000u64),
//!     true
//! ).await?;
//!
//! // Step 2: Create flow matrix
//! let matrix = create_flow_matrix(
//!     "0x123...".parse()?,
//!     "0x456...".parse()?,
//!     U192::from(1000u64),
//!     &transfers
//! )?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Modules
//!
//! - [`contract`] - Contract-compatible types and conversions
//! - [`rpc`] - RPC communication and pathfinding
//! - [`flow`] - Flow matrix calculation
//! - [`packing`] - Coordinate packing utilities
//! - [`convenience`] - High-level convenience functions
//!
//! ## Features
//!
//! - **Fast pathfinding** using Circles RPC endpoints
//! - **Contract integration** with ready-to-use types
//! - **Type safety** with `alloy-primitives`
//! - **Efficient packing** for on-chain storage
//! - **Comprehensive testing** with real-world scenarios

mod contract;
mod convenience;
mod flow;
mod packing;
mod rpc;

use alloy_primitives::aliases::U192;

// Core public API - the main functions users need
pub use flow::create_flow_matrix;

// RPC functionality
pub use rpc::{FindPathParams, find_path, find_path_with_params};

// Contract integration types and functions
pub use contract::{
    ContractFlowMatrix, FlowEdge as ContractFlowEdge, Stream as ContractStream,
    flow_matrix_to_contract_types, packed_coordinates_as_bytes,
};

// High-level convenience functions
pub use convenience::{
    get_available_flow, prepare_flow_for_contract, prepare_flow_for_contract_simple,
};

// Utility functions for advanced users
pub use packing::{pack_coordinates, transform_to_flow_vertices};

/// Errors that can occur during pathfinding and flow matrix operations.
#[derive(thiserror::Error, Debug)]
pub enum PathfinderError {
    /// Flow matrix terminal sum doesn't match expected value.
    ///
    /// This occurs when the sum of all terminal flow edges (edges that reach
    /// the destination) doesn't equal the expected transfer amount. This
    /// usually indicates an issue with the transfer path or RPC data.
    ///
    /// # Example
    /// ```text
    /// PathfinderError::Imbalanced {
    ///     terminal_sum: 800,
    ///     expected: 1000
    /// }
    /// ```
    #[error("terminal sum {terminal_sum} != expected {expected}")]
    Imbalanced {
        /// Actual sum of terminal flow edges
        terminal_sum: U192,
        /// Expected total flow amount
        expected: U192,
    },

    /// Network or HTTP communication error.
    ///
    /// This wraps underlying `reqwest` errors that occur during RPC communication.
    #[error("rpc error: {0}")]
    Rpc(#[from] reqwest::Error),

    /// JSON-RPC protocol error or invalid response.
    ///
    /// This occurs when the RPC endpoint returns an error, or when the response
    /// format doesn't match expectations (missing fields, wrong types, etc.).
    #[error("json-rpc error: {0}")]
    JsonRpc(String),
}
