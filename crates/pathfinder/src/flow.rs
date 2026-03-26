// src/flow.rs
//! Flow matrix calculation and validation.
//!
//! This module handles the conversion of transfer paths into flow matrices
//! suitable for smart contract execution, including vertex transformation,
//! edge creation, and coordinate packing.
use crate::{FlowEdge, FlowMatrix, PathfinderError, Stream};
use alloy_primitives::aliases::{U192, U256};
use alloy_primitives::{Address, Bytes};
use circles_types::TransferStep;
use std::collections::HashSet;

use crate::packing::{pack_coordinates, transform_to_flow_vertices};

fn detect_terminal_edges(transfers: &[TransferStep], receiver: Address) -> HashSet<usize> {
    let mut terminal_edges = HashSet::new();
    let mut edges_to_receiver = Vec::new();
    let mut self_loop_index = None;

    for (index, transfer) in transfers.iter().enumerate() {
        if transfer.from_address == receiver && transfer.to_address == receiver {
            self_loop_index = Some(index);
        } else if transfer.to_address == receiver {
            edges_to_receiver.push(index);
        }
    }

    if let Some(index) = self_loop_index {
        terminal_edges.insert(index);
    } else {
        terminal_edges.extend(edges_to_receiver);
    }

    terminal_edges
}

/// Create a flow matrix from a sequence of transfer steps.
///
/// This function takes a path discovered by [`crate::find_path`] and converts it into
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
/// - [`crate::find_path`] - For discovering transfer paths
/// - [`crate::prepare_flow_for_contract`] - For one-step path finding + matrix creation
pub fn create_flow_matrix(
    sender: Address,
    receiver: Address,
    value: U192,
    transfers: &[TransferStep],
) -> Result<FlowMatrix, PathfinderError> {
    if transfers.is_empty() {
        // If the TS version never calls this with an empty path,
        // treat it as a logic error / invalid input:
        return Err(PathfinderError::Imbalanced {
            expected: value,
            terminal_sum: U192::from(0u64),
        });
    }

    let (flow_vertices, idx) = transform_to_flow_vertices(transfers, sender, receiver);
    let terminal_edge_indices = detect_terminal_edges(transfers, receiver);

    // Build edges
    let flow_edges: Vec<FlowEdge> = transfers
        .iter()
        .enumerate()
        .map(|(index, t)| FlowEdge {
            streamSinkId: if terminal_edge_indices.contains(&index) {
                1
            } else {
                0
            },
            amount: t.value,
        })
        .collect();

    if terminal_edge_indices.is_empty() {
        return Err(PathfinderError::RpcResponse(format!(
            "No terminal edges detected. Flow must have at least one edge delivering to receiver {receiver:#x}"
        )));
    }

    // Check terminal balance
    let terminal_sum: U192 = flow_edges
        .iter()
        .filter(|e| e.streamSinkId == 1)
        .map(|e| e.amount)
        .sum();
    if terminal_sum != value {
        return Err(PathfinderError::Imbalanced {
            terminal_sum,
            expected: value,
        });
    }

    // Build streams
    let mut term_edge_ids: Vec<u16> = terminal_edge_indices
        .iter()
        .map(|index| *index as u16)
        .collect();
    term_edge_ids.sort_unstable();

    let streams = vec![Stream {
        sourceCoordinate: *idx.get(&sender).unwrap() as u16,
        flowEdgeIds: term_edge_ids,
        data: Bytes::new(),
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
        source_coordinate: U256::from(*idx.get(&sender).unwrap()),
    })
}

/// Clone flow-matrix streams and optionally attach transaction data to the first stream.
///
/// This mirrors the TypeScript helper used before contract submission, but keeps
/// the Rust contract-facing `Bytes` representation instead of hex strings.
pub fn prepare_flow_matrix_streams(
    flow_matrix: &FlowMatrix,
    tx_data: Option<Bytes>,
) -> Vec<Stream> {
    let mut streams = flow_matrix.streams.clone();
    if let Some(tx_data) = tx_data
        && let Some(first) = streams.first_mut()
    {
        first.data = tx_data;
    }
    streams
}
