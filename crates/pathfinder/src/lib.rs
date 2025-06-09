mod flow;
mod packing;
mod rpc;
mod contract;
mod convenience;

use alloy_primitives::U256;

// Core public API - the main functions users need
pub use flow::create_flow_matrix;

// RPC functionality
pub use rpc::{find_path, find_path_with_params, FindPathParams};

// Contract integration types and functions
pub use contract::{
    ContractFlowMatrix, 
    FlowEdge as ContractFlowEdge, 
    Stream as ContractStream,
    flow_matrix_to_contract_types,
    packed_coordinates_as_bytes
};

// High-level convenience functions
pub use convenience::{
    prepare_flow_for_contract, 
    prepare_flow_for_contract_simple,
    get_available_flow
};

// Utility functions for advanced users
pub use packing::{pack_coordinates, transform_to_flow_vertices};

#[derive(thiserror::Error, Debug)]
pub enum PathfinderError {
    #[error("terminal sum {terminal_sum} != expected {expected}")]
    Imbalanced { terminal_sum: U256, expected: U256 },
    #[error("rpc error: {0}")]
    Rpc(#[from] reqwest::Error),
    #[error("json-rpc error: {0}")]
    JsonRpc(String),
}
