use alloy_primitives::aliases::U192;
use alloy_primitives::{Address, Bytes};
use circles_types::{FlowEdge as PathfinderFlowEdge, FlowMatrix, Stream as PathfinderStream};

/// Contract-compatible FlowEdge type matching the smart contract ABI
#[derive(Debug, Clone, PartialEq)]
pub struct FlowEdge {
    /// Stream sink ID (uint16 in contract)
    pub stream_sink_id: u16,
    /// Amount (uint192 in contract, but U256 is compatible)
    pub amount: U192,
}

/// Contract-compatible Stream type matching the smart contract ABI
#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    /// Source coordinate (uint16 in contract)
    pub source_coordinate: u16,
    /// Flow edge IDs (uint16[] in contract)
    pub flow_edge_ids: Vec<u16>,
    /// Additional data (bytes in contract)
    pub data: Bytes,
}

/// Complete flow matrix with contract-compatible types.
///
/// This struct contains all data needed for smart contract function calls,
/// with types that exactly match the expected contract ABI. All internal
/// pathfinder types are automatically converted to contract-compatible formats.
///
/// # Contract ABI Compatibility
///
/// This struct maps to the following Solidity interface:
/// ```solidity
/// struct FlowEdge {
///     uint16 streamSinkId;
///     uint192 amount;
/// }
///
/// struct Stream {
///     uint16 sourceCoordinate;
///     uint16[] flowEdgeIds;
///     bytes data;
/// }
///
/// function transferFlow(
///     address[] memory flowVertices,
///     FlowEdge[] memory flowEdges,
///     Stream[] memory streams,
///     bytes memory packedCoordinates
/// ) external;
/// ```
///
/// # Examples
///
/// ```rust,ignore
/// use circles_pathfinder::prepare_flow_for_contract;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let rpc_url = "https://rpc.aboutcircles.com/";
/// # let params = circles_pathfinder::FindPathParams {
/// #     from: "0x52e14be00d5acff4424ad625662c6262b4fd1a58".parse()?,
/// #     to: "0xcf6dc192dc292d5f2789da2db02d6dd4f41f4214".parse()?,
/// #     target_flow: alloy_primitives::U256::from(1000u64),
/// #     use_wrapped_balances: Some(true),
/// #     from_tokens: None, to_tokens: None, exclude_from_tokens: None, exclude_to_tokens: None,
/// # };
/// let matrix = prepare_flow_for_contract(rpc_url, params).await?;
///
/// // Direct usage with contract call
/// let tx = contract.transferFlow(
///     matrix.flow_vertices,
///     matrix.flow_edges,
///     matrix.streams,
///     matrix.packed_coordinates
/// ).send().await?;
///
/// // Or decompose for tuple-based calls
/// let (vertices, edges, streams, coords) = matrix.into_contract_params();
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ContractFlowMatrix {
    /// Sorted list of all addresses involved in the flow (address[] in contract)
    pub flow_vertices: Vec<Address>,
    /// Flow edges with contract-compatible types (FlowEdge[] in contract)
    pub flow_edges: Vec<FlowEdge>,
    /// Streams with contract-compatible types (Stream[] in contract)
    pub streams: Vec<Stream>,
    /// Packed coordinates as bytes (bytes in contract)
    pub packed_coordinates: Bytes,
    /// Source coordinate index
    pub source_coordinate: u16,
}

impl ContractFlowMatrix {
    /// Create a new ContractFlowMatrix from individual components
    pub fn new(
        flow_vertices: Vec<Address>,
        flow_edges: Vec<FlowEdge>,
        streams: Vec<Stream>,
        packed_coordinates: Vec<u8>,
        source_coordinate: u16,
    ) -> Self {
        Self {
            flow_vertices,
            flow_edges,
            streams,
            packed_coordinates: Bytes::from(packed_coordinates),
            source_coordinate,
        }
    }

    /// Decompose into tuple format often used in contract calls
    /// Returns: (flow_vertices, flow_edges, streams, packed_coordinates)
    pub fn into_contract_params(self) -> (Vec<Address>, Vec<FlowEdge>, Vec<Stream>, Bytes) {
        (
            self.flow_vertices,
            self.flow_edges,
            self.streams,
            self.packed_coordinates,
        )
    }
}

// Conversion from our internal FlowMatrix to contract-compatible types
impl From<FlowMatrix> for ContractFlowMatrix {
    fn from(matrix: FlowMatrix) -> Self {
        Self {
            flow_vertices: matrix.flow_vertices,
            flow_edges: matrix.flow_edges.into_iter().map(Into::into).collect(),
            streams: matrix.streams.into_iter().map(Into::into).collect(),
            packed_coordinates: Bytes::from(matrix.packed_coordinates),
            source_coordinate: matrix.source_coordinate,
        }
    }
}

// Conversion from internal FlowEdge to contract FlowEdge
impl From<PathfinderFlowEdge> for FlowEdge {
    fn from(edge: PathfinderFlowEdge) -> Self {
        Self {
            stream_sink_id: edge.stream_sink_id,
            amount: edge.amount,
        }
    }
}

// Conversion from internal Stream to contract Stream
impl From<PathfinderStream> for Stream {
    fn from(stream: PathfinderStream) -> Self {
        Self {
            source_coordinate: stream.source_coordinate,
            flow_edge_ids: stream.flow_edge_ids,
            data: Bytes::from(stream.data),
        }
    }
}

// Convenience functions for FlowMatrix conversion
/// Convert a FlowMatrix to contract-compatible types
///
/// This is a convenience function that converts all internal types
/// to types that can be directly used with smart contract calls.
pub fn flow_matrix_to_contract_types(matrix: FlowMatrix) -> ContractFlowMatrix {
    matrix.into()
}

/// Get packed coordinates as Bytes for contract calls
///
/// Returns the packed coordinates in a format ready for smart contract calls.
pub fn packed_coordinates_as_bytes(packed_coordinates: &[u8]) -> Bytes {
    Bytes::from(packed_coordinates.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::aliases::U192;

    #[test]
    fn test_flow_edge_conversion() {
        let internal_edge = PathfinderFlowEdge {
            stream_sink_id: 1,
            amount: U192::from(1000u64),
        };

        let contract_edge: FlowEdge = internal_edge.into();

        assert_eq!(contract_edge.stream_sink_id, 1);
        assert_eq!(contract_edge.amount, U192::from(1000u64));
    }

    #[test]
    fn test_stream_conversion() {
        let internal_stream = PathfinderStream {
            source_coordinate: 0,
            flow_edge_ids: vec![1, 2, 3],
            data: vec![0x01, 0x02, 0x03],
        };

        let contract_stream: Stream = internal_stream.into();

        assert_eq!(contract_stream.source_coordinate, 0);
        assert_eq!(contract_stream.flow_edge_ids, vec![1, 2, 3]);
        assert_eq!(contract_stream.data, Bytes::from(vec![0x01, 0x02, 0x03]));
    }

    #[test]
    fn test_contract_flow_matrix_decomposition() {
        let flow_vertices = vec![Address::ZERO];
        let flow_edges = vec![FlowEdge {
            stream_sink_id: 1,
            amount: U192::from(1000u64),
        }];
        let streams = vec![Stream {
            source_coordinate: 0,
            flow_edge_ids: vec![0],
            data: Bytes::new(),
        }];
        let packed_coordinates = Bytes::from(vec![0x01, 0x02]);

        let matrix = ContractFlowMatrix {
            flow_vertices: flow_vertices.clone(),
            flow_edges: flow_edges.clone(),
            streams: streams.clone(),
            packed_coordinates: packed_coordinates.clone(),
            source_coordinate: 0,
        };

        let (vertices, edges, streams_out, coords) = matrix.into_contract_params();

        assert_eq!(vertices, flow_vertices);
        assert_eq!(edges, flow_edges);
        assert_eq!(streams_out, streams);
        assert_eq!(coords, packed_coordinates);
    }
}
