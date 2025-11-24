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
//! use circles_types::{TransferStep, FlowEdge, Stream, FindPathParams};
//! use alloy_primitives::{Address, U256};
//!
//! // Create a transfer step
//! let transfer = TransferStep {
//!     from_address: "0x123...".parse()?,
//!     to_address: "0x456...".parse()?,
//!     token_owner: "0x789...".parse()?,
//!     value: U256::from(1000u64).into(),
//! };
//!
//! // Create pathfinding parameters
//! let params = FindPathParams {
//!     from: "0xabc...".parse()?,
//!     to: "0xdef...".parse()?,
//!     target_flow: U256::from(1000u64),
//!     use_wrapped_balances: Some(true),
//!     from_tokens: None,
//!     to_tokens: None,
//!     exclude_from_tokens: None,
//!     exclude_to_tokens: None,
//!     simulated_balances: None,
//!     max_transfers: Some(10),
//! };
//!
//! // Serialize to JSON
//! let json = serde_json::to_string(&params)?;
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

mod base;
pub use base::*;

mod avatar;
pub use avatar::*;

mod config;
pub use config::*;

mod contracts;
pub use contracts::*;

mod errors;
pub use errors::*;

mod events;
pub use events::*;

mod group;
pub use group::*;

mod network;
pub use network::*;

mod trust;
pub use trust::*;

mod wrapper;
pub use wrapper::*;

mod token;
pub use token::*;

mod client;
pub use client::*;

mod runner;
pub use runner::*;

mod rpc;
pub use rpc::*;

mod query;
pub use query::*;

mod pathfinding;
pub use pathfinding::*;

pub type Address = alloy_primitives::Address;
