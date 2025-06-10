// src/flow.rs
//! Flow matrix calculation and validation.
//!
//! This module handles the conversion of transfer paths into flow matrices
//! suitable for smart contract execution, including vertex transformation,
//! edge creation, and coordinate packing.
use crate::PathfinderError;
use alloy_primitives::Address;
use alloy_primitives::aliases::U192;
use circles_types::{FlowEdge, FlowMatrix, Stream, TransferStep};

use crate::packing::{pack_coordinates, transform_to_flow_vertices};

/// Create a flow matrix from a sequence of transfer steps.
///
/// This function takes a path discovered by [`find_path`] and converts it into
/// a flow matrix suitable for smart contract execution. The matrix includes
/// vertex coordinates, flow edges, streams, and packed coordinate data.
///
/// # Arguments
///
/// * `sender` - Source address for the flow
/// * `receiver` - Destination address for the flow
/// * `value` - Expected total value to be transferred to receiver
/// * `transfers` - Sequence of transfer steps from pathfinding
///
/// # Returns
///
/// Returns a [`FlowMatrix`] containing all necessary data for contract calls,
/// or an error if the transfers don't balance correctly.
///
/// # Examples
///
/// ```rust
/// use circles_pathfinder::{find_path, create_flow_matrix};
/// use alloy_primitives::{Address, aliases::U192};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let from: Address = "0x123...".parse()?;
/// let to: Address = "0x456...".parse()?;
/// let amount = U192::from(1000u64);
///
/// // First find a path
/// let transfers = find_path("https://rpc.circles.com", from, to, amount, true).await?;
///
/// // Then create the flow matrix
/// let matrix = create_flow_matrix(from, to, amount, &transfers)?;
///
/// println!("Matrix has {} vertices", matrix.flow_vertices.len());
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// - [`PathfinderError::Imbalanced`] - When terminal flow doesn't match expected value
///
/// # See Also
///
/// - [`find_path`] - For discovering transfer paths
/// - [`prepare_flow_for_contract`] - For one-step path finding + matrix creation
pub fn create_flow_matrix(
    sender: Address,
    receiver: Address,
    value: U192,
    transfers: &[TransferStep],
) -> Result<FlowMatrix, PathfinderError> {
    let (flow_vertices, idx) = transform_to_flow_vertices(transfers, sender, receiver);

    // Build edges
    let mut flow_edges: Vec<FlowEdge> = transfers
        .iter()
        .map(|t| FlowEdge {
            stream_sink_id: if t.to_address == receiver { 1 } else { 0 },
            amount: t.value,
        })
        .collect();

    // Ensure at least one terminal edge
    if !flow_edges.iter().any(|e| e.stream_sink_id == 1) {
        let fallback = transfers
            .iter()
            .rposition(|t| t.to_address == receiver)
            .unwrap_or(flow_edges.len() - 1);
        flow_edges[fallback].stream_sink_id = 1;
    }

    // Check terminal balance
    let terminal_sum: U192 = flow_edges
        .iter()
        .filter(|e| e.stream_sink_id == 1)
        .map(|e| e.amount)
        .sum();
    if terminal_sum != value {
        return Err(PathfinderError::Imbalanced {
            terminal_sum,
            expected: value,
        });
    }

    // Build streams
    let term_edge_ids: Vec<u16> = flow_edges
        .iter()
        .enumerate()
        .filter_map(|(i, e)| (e.stream_sink_id == 1).then_some(i as u16))
        .collect();

    let streams = vec![Stream {
        source_coordinate: *idx.get(&sender).unwrap() as u16,
        flow_edge_ids: term_edge_ids,
        data: Vec::new(),
    }];

    // Pack coordinates
    let mut coords: Vec<u16> = Vec::with_capacity(transfers.len() * 3);
    for t in transfers {
        coords.push(*idx.get(&t.token_owner).unwrap() as u16);
        coords.push(*idx.get(&t.from_address).unwrap() as u16);
        coords.push(*idx.get(&t.to_address).unwrap() as u16);
    }
    let packed_coordinates = pack_coordinates(&coords);

    Ok(FlowMatrix {
        flow_vertices,
        flow_edges,
        streams,
        packed_coordinates,
        source_coordinate: *idx.get(&sender).unwrap() as u16,
    })
}
