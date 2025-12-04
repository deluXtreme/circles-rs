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
//! use alloy_primitives::{Address, aliases::U192, U256};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let params = FindPathParams {
//!     from: "0x123...".parse()?,
//!     to: "0x456...".parse()?,
//!     target_flow: U256::from(1_000_000_000_000_000_000u64), // 1 CRC
//!     use_wrapped_balances: Some(true),
//!     from_tokens: None,
//!     to_tokens: None,
//!     exclude_from_tokens: None,
//!     exclude_to_tokens: None,
//!     simulated_balances: None,
//!     max_transfers: None,
//! };
//!
//! // One function call gets contract-ready data
//! let path_data = prepare_flow_for_contract("https://rpc.circles.com", params).await?;
//!
//! // Ready for smart contract calls
//! let (vertices, edges, streams, coords) = path_data.to_contract_params();
//! // contract.some_function(vertices, edges, streams, coords)
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
//! - `hub` - Circles Hub contract types and conversions
//! - `rpc` - RPC communication and pathfinding
//! - `flow` - Flow matrix calculation
//! - `packing` - Coordinate packing utilities
//! - `convenience` - High-level convenience functions
//!
//! ## Features
//!
//! - **Fast pathfinding** using Circles RPC endpoints
//! - **Hub contract integration** with ready-to-use types
//! - **Type safety** with `alloy-primitives`
//! - **Efficient packing** for on-chain storage
//! - **Comprehensive testing** with real-world scenarios

mod convenience;
mod flow;
pub mod hub;
mod packing;
mod rpc;

use alloy_primitives::{U256, aliases::U192};

// Core public API - the main functions users need
pub use flow::create_flow_matrix;
pub mod path;

// RPC functionality
pub use circles_types::FindPathParams;
pub use rpc::{find_path, find_path_with_params};

// Hub contract integration types and functions
use alloy_primitives::Address;
use alloy_sol_types::sol;
pub use hub::PathData;

// High-level convenience functions
pub use convenience::{
    encode_redeem_flow_matrix, encode_redeem_trusted_data, get_available_flow,
    prepare_flow_for_contract, prepare_flow_for_contract_simple,
};

pub use path::{
    assert_no_netted_flow_mismatch, compute_netted_flow, expected_unwrapped_totals,
    replace_wrapped_tokens, shrink_path_values, token_info_map_from_path, wrapped_totals_from_path,
};

// Utility functions for advanced users
pub use packing::{pack_coordinates, transform_to_flow_vertices};

#[derive(Clone, Debug)]
pub struct FlowMatrix {
    pub flow_vertices: Vec<Address>,
    pub flow_edges: Vec<FlowEdge>,
    pub streams: Vec<Stream>,
    pub packed_coordinates: Vec<u8>,
    pub source_coordinate: U256,
}

sol!(
    /// Standard Circles Hub FlowEdge struct matching the contract ABI
    #[derive(Debug, PartialEq)]
    struct FlowEdge {
        uint16 streamSinkId;
        uint192 amount;
    }

    /// Standard Circles Hub Stream struct matching the contract ABI
    #[derive(Debug, PartialEq)]
    struct Stream {
        uint16 sourceCoordinate;
        uint16[] flowEdgeIds;
        bytes data;
    }

    function redeem(bytes32 id, bytes calldata data) external;
);

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

    /// RPC transport/client error (HTTP/WS or deserialization).
    #[error("rpc transport error: {0}")]
    Transport(#[from] circles_rpc::CirclesRpcError),

    /// JSON-RPC payload error returned by the server or an invalid response body.
    #[error("rpc response error: {0}")]
    RpcResponse(String),
}
