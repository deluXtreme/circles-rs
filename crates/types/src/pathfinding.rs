use alloy_primitives::{aliases::U192, Address, Bytes, U256};
use serde::{Deserialize, Serialize};

/// Simulated balance for path finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedBalance {
    pub holder: Address,
    pub token: Address,
    pub amount: U256,
    pub is_wrapped: bool,
    pub is_static: bool,
}

/// Path finding parameters for circlesV2_findPath
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindPathParams {
    pub from: Address,
    pub to: Address,
    pub target_flow: U256,
    pub use_wrapped_balances: Option<bool>,
    pub from_tokens: Option<Vec<Address>>,
    pub to_tokens: Option<Vec<Address>>,
    pub exclude_from_tokens: Option<Vec<Address>>,
    pub exclude_to_tokens: Option<Vec<Address>>,
    pub simulated_balances: Option<Vec<SimulatedBalance>>,
    pub max_transfers: Option<u32>,
}

/// A single transfer step in a pathfinding result
/// This is the pathfinding version - different from the existing TransferStep in lib.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathfindingTransferStep {
    pub from: Address,
    pub to: Address,
    pub token_owner: String, // TypeScript uses string, keeping it for API compatibility
    pub value: U256,
}

/// Result of pathfinding computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathfindingResult {
    pub max_flow: U256,
    pub transfers: Vec<PathfindingTransferStep>,
}

/// Flow edge structure for operateFlowMatrix
/// Corresponds to TypeDefinitions.FlowEdge in the Hub V2 contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEdgeStruct {
    #[serde(rename = "streamSinkId")]
    pub stream_sink_id: u16,
    pub amount: U192, // uint192 in Solidity
}

/// Stream structure for operateFlowMatrix
/// Corresponds to TypeDefinitions.Stream in the Hub V2 contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStruct {
    #[serde(rename = "sourceCoordinate")]
    pub source_coordinate: u16,
    #[serde(rename = "flowEdgeIds")]
    pub flow_edge_ids: Vec<u16>,
    pub data: Bytes, // Handles both Uint8Array and Hex from TypeScript
}

/// Flow matrix for ABI encoding
/// Used with the operateFlowMatrix function in Hub V2
/// This is the pathfinding version - different from the existing FlowMatrix in lib.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathfindingFlowMatrix {
    #[serde(rename = "flowVertices")]
    pub flow_vertices: Vec<String>, // Keep as strings for API compatibility
    #[serde(rename = "flowEdges")]
    pub flow_edges: Vec<FlowEdgeStruct>,
    pub streams: Vec<StreamStruct>,
    #[serde(rename = "packedCoordinates")]
    pub packed_coordinates: String, // Hex string
    #[serde(rename = "sourceCoordinate")]
    pub source_coordinate: u16, // Convenience field, not part of ABI
}

/// Advanced transfer options
/// Extends FindPathParams to add transfer-specific options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedTransferOptions {
    // All fields from FindPathParams except from, to, targetFlow
    pub use_wrapped_balances: Option<bool>,
    pub from_tokens: Option<Vec<Address>>,
    pub to_tokens: Option<Vec<Address>>,
    pub exclude_from_tokens: Option<Vec<Address>>,
    pub exclude_to_tokens: Option<Vec<Address>>,
    pub simulated_balances: Option<Vec<SimulatedBalance>>,
    pub max_transfers: Option<u32>,

    /// Custom data to attach to the transfer (optional)
    pub tx_data: Option<Bytes>,
}

impl AdvancedTransferOptions {
    /// Convert to FindPathParams with the required from/to/targetFlow fields
    pub fn to_find_path_params(
        self,
        from: Address,
        to: Address,
        target_flow: U256,
    ) -> FindPathParams {
        FindPathParams {
            from,
            to,
            target_flow,
            use_wrapped_balances: self.use_wrapped_balances,
            from_tokens: self.from_tokens,
            to_tokens: self.to_tokens,
            exclude_from_tokens: self.exclude_from_tokens,
            exclude_to_tokens: self.exclude_to_tokens,
            simulated_balances: self.simulated_balances,
            max_transfers: self.max_transfers,
        }
    }
}

// ============================================================================
// Original Flow Types (moved from lib.rs)
// ============================================================================

/// Edge in the flow graph (sinkId 1 == final hop)
/// This is the original version from lib.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlowEdge {
    pub stream_sink_id: u16,
    pub amount: U192,
}

/// Stream with byte data
/// This is the original version from lib.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stream {
    pub source_coordinate: u16,
    pub flow_edge_ids: Vec<u16>,
    pub data: Vec<u8>,
}

/// Transfer step for internal flow calculations
/// This is the original version from lib.rs - different from PathfindingTransferStep
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferStep {
    pub from_address: Address,
    pub to_address: Address,
    pub token_owner: Address,
    pub value: U192,
}

/// ABI-ready matrix returned by `create_flow_matrix`
/// This is the original version from lib.rs - different from PathfindingFlowMatrix
#[derive(Clone, Debug)]
pub struct FlowMatrix {
    pub flow_vertices: Vec<Address>,
    pub flow_edges: Vec<FlowEdge>,
    pub streams: Vec<Stream>,
    pub packed_coordinates: Vec<u8>,
    pub source_coordinate: u16,
}
