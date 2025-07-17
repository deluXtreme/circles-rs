//! # Circles Hub Contract Integration
//!
//! This module provides standard Circles Hub contract types and utilities for
//! converting pathfinding results into contract-compatible formats.
//!
//! The types defined here match the exact ABI of the Circles Hub smart contract,
//! ensuring seamless integration with contract calls.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use circles_pathfinder::{find_path_with_params, FindPathParams, PathData};
//! use alloy_primitives::Address;
//! use alloy_primitives::aliases::U192;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let params = FindPathParams {
//!     from: Address::ZERO,
//!     to: Address::from([1u8; 20]),
//!     target_flow: U192::from(1000u64),
//!     use_wrapped_balances: Some(true),
//!     from_tokens: None,
//!     to_tokens: None,
//!     exclude_from_tokens: None,
//!     exclude_to_tokens: None,
//! };
//!
//! // Find the path
//! let transfers = find_path_with_params("https://rpc.aboutcircles.com", params.clone()).await?;
//!
//! // Create PathData and convert to contract types
//! let path_data = PathData::from_transfers(&transfers, params.from, params.to, params.target_flow)?;
//! let (vertices, edges, streams, coords) = path_data.to_contract_params();
//!
//! // Ready for contract calls!
//! // contract.some_function(vertices, edges, streams, coords).send().await?;
//! # Ok(())
//! # }
//! ```

use crate::{FlowEdge, FlowMatrix, PathfinderError, Stream, create_flow_matrix};
use alloy_primitives::aliases::U192;
use alloy_primitives::{Address, Bytes, U256};
use circles_types::TransferStep;

/// Simplified pathfinding result data structure
///
/// This struct contains the raw pathfinding results in a format that can be
/// easily converted to contract-compatible types. It eliminates the need for
/// manual field-by-field conversion between different type layers.
#[derive(Debug, Clone)]
pub struct PathData {
    /// Sorted list of all addresses involved in the flow
    pub flow_vertices: Vec<Address>,
    /// Flow edges as (stream_sink_id, amount) tuples
    pub flow_edges: Vec<FlowEdge>, // Vec<(u16, U192)>,
    /// Streams as (source_coordinate, flow_edge_ids, data) tuples
    pub streams: Vec<Stream>, // Vec<(u16, Vec<u16>, Vec<u8>)>,
    /// Packed coordinates as raw bytes
    pub packed_coordinates: Vec<u8>,
    /// Source coordinate index
    pub source_coordinate: U256,
}

impl PathData {
    /// Create PathData from transfer steps
    ///
    /// This is the main constructor that takes the output from pathfinding
    /// and creates a PathData structure ready for contract conversion.
    ///
    /// # Arguments
    /// * `transfers` - Vector of transfer steps from pathfinding
    /// * `from` - Source address
    /// * `to` - Destination address
    /// * `target_flow` - Target flow amount
    ///
    /// # Returns
    /// A PathData structure ready for contract conversion
    ///
    /// # Errors
    /// Returns PathfinderError if flow matrix creation fails
    pub fn from_transfers(
        transfers: &[TransferStep],
        from: Address,
        to: Address,
        target_flow: U192,
    ) -> Result<Self, PathfinderError> {
        // Calculate actual available flow
        let actual_flow: U192 = transfers
            .iter()
            .filter(|t| t.to_address == to)
            .map(|t| t.value)
            .sum();

        // Use the smaller of target_flow or actual available flow
        let flow_amount = if actual_flow < target_flow {
            actual_flow
        } else {
            target_flow
        };

        // Create flow matrix
        let matrix = create_flow_matrix(from, to, flow_amount, transfers)?;

        Ok(Self::from_flow_matrix(matrix))
    }

    /// Create PathData from a FlowMatrix
    ///
    /// Internal constructor for converting from the core FlowMatrix type.
    fn from_flow_matrix(matrix: FlowMatrix) -> Self {
        Self {
            flow_vertices: matrix.flow_vertices,
            flow_edges: matrix.flow_edges,
            streams: matrix.streams,
            packed_coordinates: matrix.packed_coordinates,
            source_coordinate: matrix.source_coordinate,
        }
    }

    /// Convert to standard Circles Hub FlowEdge types
    ///
    /// Returns a vector of FlowEdge structs with the exact field names
    /// and types expected by the Circles Hub smart contract.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use circles_pathfinder::hub::PathData;
    /// # use alloy_primitives::aliases::U192;
    /// # use alloy_primitives::U256;
    /// # use circles_pathfinder::FlowEdge;
    /// # let path_data = PathData {
    /// #     flow_vertices: vec![],
    /// #     flow_edges: vec![FlowEdge { streamSinkId: 1, amount: U192::from(1000u64) }],
    /// #     streams: vec![],
    /// #     packed_coordinates: vec![],
    /// #     source_coordinate: U256::from(0),
    /// # };
    /// let edges = path_data.to_flow_edges();
    /// assert_eq!(edges[0].streamSinkId, 1);
    /// assert_eq!(edges[0].amount, U192::from(1000u64));
    /// ```
    pub fn to_flow_edges(&self) -> Vec<FlowEdge> {
        self.flow_edges
            .iter()
            .map(|flow_edge| FlowEdge {
                streamSinkId: flow_edge.streamSinkId,
                amount: flow_edge.amount,
            })
            .collect()
    }

    /// Convert to standard Circles Hub Stream types
    ///
    /// Returns a vector of Stream structs with the exact field names
    /// and types expected by the Circles Hub smart contract.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use circles_pathfinder::hub::PathData;
    /// # use circles_pathfinder::Stream;
    /// # use alloy_primitives::U256;
    /// # let path_data = PathData {
    /// #     flow_vertices: vec![],
    /// #     flow_edges: vec![],
    /// #     streams: vec![Stream { sourceCoordinate:0, flowEdgeIds: vec![1, 2], data: vec![0x01, 0x02].into(),}],
    /// #     packed_coordinates: vec![],
    /// #     source_coordinate: U256::from(0),
    /// # };
    /// let streams = path_data.to_streams();
    /// assert_eq!(streams[0].sourceCoordinate, 0);
    /// assert_eq!(streams[0].flowEdgeIds, vec![1, 2]);
    /// ```
    pub fn to_streams(&self) -> Vec<Stream> {
        self.streams
            .iter()
            .map(|stream| Stream {
                sourceCoordinate: stream.sourceCoordinate,
                flowEdgeIds: stream.flowEdgeIds.clone(),
                data: stream.data.clone(),
            })
            .collect()
    }

    /// Get all contract call parameters in one tuple
    ///
    /// This convenience method returns all the parameters needed for most
    /// Circles Hub contract calls in the correct order and format.
    ///
    /// # Returns
    /// A tuple of (flow_vertices, flow_edges, streams, packed_coordinates)
    /// ready to use in contract function calls.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use circles_pathfinder::hub::PathData;
    /// # use alloy_primitives::U256;
    /// # let path_data = PathData {
    /// #     flow_vertices: vec![],
    /// #     flow_edges: vec![],
    /// #     streams: vec![],
    /// #     packed_coordinates: vec![0x01, 0x02],
    /// #     source_coordinate: U256::from(0),
    /// # };
    /// let (vertices, edges, streams, coords) = path_data.to_contract_params();
    ///
    /// // Ready for contract calls:
    /// // contract.transferFlow(vertices, edges, streams, coords).send().await?;
    /// ```
    pub fn to_contract_params(&self) -> (Vec<Address>, Vec<FlowEdge>, Vec<Stream>, Bytes) {
        (
            self.flow_vertices.clone(),
            self.to_flow_edges(),
            self.to_streams(),
            Bytes::from(self.packed_coordinates.clone()),
        )
    }

    /// Get packed coordinates as Bytes
    ///
    /// Convenience method to get the packed coordinates in the Bytes format
    /// expected by contract calls.
    pub fn to_packed_coordinates(&self) -> Bytes {
        Bytes::from(self.packed_coordinates.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::aliases::U192;

    #[test]
    fn test_flow_edge_creation() {
        let edge = FlowEdge {
            streamSinkId: 1,
            amount: U192::from(1000u64),
        };

        assert_eq!(edge.streamSinkId, 1);
        assert_eq!(edge.amount, U192::from(1000u64));
    }

    #[test]
    fn test_stream_creation() {
        let stream = Stream {
            sourceCoordinate: 0,
            flowEdgeIds: vec![1, 2, 3],
            data: Bytes::from(vec![0x01, 0x02, 0x03]),
        };

        assert_eq!(stream.sourceCoordinate, 0);
        assert_eq!(stream.flowEdgeIds, vec![1, 2, 3]);
        assert_eq!(stream.data, Bytes::from(vec![0x01, 0x02, 0x03]));
    }

    // #[test]
    // fn test_path_data_conversions() {
    //     let path_data = PathData {
    //         flow_vertices: vec![Address::ZERO],
    //         flow_edges: vec![(1, U192::from(1000u64))],
    //         streams: vec![(0, vec![0], vec![0x01, 0x02])],
    //         packed_coordinates: vec![0x03, 0x04],
    //         source_coordinate: 0,
    //     };

    //     // Test individual conversions
    //     let edges = path_data.to_flow_edges();
    //     assert_eq!(edges.len(), 1);
    //     assert_eq!(edges[0].streamSinkId, 1);
    //     assert_eq!(edges[0].amount, U192::from(1000u64));

    //     let streams = path_data.to_streams();
    //     assert_eq!(streams.len(), 1);
    //     assert_eq!(streams[0].sourceCoordinate, 0);
    //     assert_eq!(streams[0].flowEdgeIds, vec![0]);
    //     assert_eq!(streams[0].data, Bytes::from(vec![0x01, 0x02]));

    //     // Test combined conversion
    //     let (vertices, edges, streams, coords) = path_data.to_contract_params();
    //     assert_eq!(vertices, vec![Address::ZERO]);
    //     assert_eq!(edges.len(), 1);
    //     assert_eq!(streams.len(), 1);
    //     assert_eq!(coords, Bytes::from(vec![0x03, 0x04]));
    // }

    #[test]
    fn test_packed_coordinates_conversion() {
        let path_data = PathData {
            flow_vertices: vec![],
            flow_edges: vec![],
            streams: vec![],
            packed_coordinates: vec![0x01, 0x02, 0x03],
            source_coordinate: U256::from(0),
        };

        let coords = path_data.to_packed_coordinates();
        assert_eq!(coords, Bytes::from(vec![0x01, 0x02, 0x03]));
    }
}
