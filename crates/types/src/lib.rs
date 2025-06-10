//! # Circles Types
//!
//! Core type definitions for the Circles protocol ecosystem.
//!
//! This crate provides fundamental data structures used throughout the Circles
//! protocol implementation, including flow matrices, transfer steps, and address
//! handling with full `serde` serialization support.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use circles_types::{TransferStep, FlowEdge, Stream};
//! use alloy_primitives::U256;
//!
//! // Create a transfer step
//! let transfer = TransferStep {
//!     from_address: "0x123...".parse()?,
//!     to_address: "0x456...".parse()?,
//!     token_owner: "0x789...".parse()?,
//!     value: U256::from(1000u64),
//! };
//!
//! // Serialize to JSON
//! let json = serde_json::to_string(&transfer)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Core Types
//!
//! - [`TransferStep`] - Single transfer operation in a multi-hop path
//! - [`FlowEdge`] - Directed edge in flow graph with routing info
//! - [`Stream`] - Collection of edges representing a transfer route
//! - [`FlowMatrix`] - Complete flow representation for contracts
//! - [`Address`] - Ethereum address (re-exported from alloy-primitives)
use alloy_primitives::aliases::U192;
use serde::{Deserialize, Serialize};

pub type Address = alloy_primitives::Address;

/// Edge in the flow graph (sinkId 1 == final hop)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlowEdge {
    pub stream_sink_id: u16,
    pub amount: U192,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stream {
    pub source_coordinate: u16,
    pub flow_edge_ids: Vec<u16>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferStep {
    pub from_address: Address,
    pub to_address: Address,
    pub token_owner: Address,
    pub value: U192,
}

/// ABI-ready matrix returned by `create_flow_matrix`
#[derive(Clone, Debug)]
pub struct FlowMatrix {
    pub flow_vertices: Vec<Address>,
    pub flow_edges: Vec<FlowEdge>,
    pub streams: Vec<Stream>,
    pub packed_coordinates: Vec<u8>,
    pub source_coordinate: u16,
}
