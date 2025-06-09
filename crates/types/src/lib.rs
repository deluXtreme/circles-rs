use alloy_primitives::U256;
use serde::{Deserialize, Serialize};

pub type Address = alloy_primitives::Address;

/// Edge in the flow graph (sinkId 1 == final hop)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlowEdge {
    pub stream_sink_id: u16,
    pub amount: U256,
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
    pub value: U256,
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
