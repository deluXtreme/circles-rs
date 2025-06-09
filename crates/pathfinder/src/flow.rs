use crate::PathfinderError;
use alloy_primitives::{Address, U256};
use types::{FlowEdge, FlowMatrix, Stream, TransferStep};

use crate::packing::{pack_coordinates, transform_to_flow_vertices};

/// Build the matrix → identical arithmetic to TS `createFlowMatrix` (https://github.com/aboutcircles/circles-sdk/raw/dev/packages/pathfinder/src/flowMatrix.ts)
pub fn create_flow_matrix(
    sender: Address,
    receiver: Address,
    value: U256,
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
    let terminal_sum: U256 = flow_edges
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
