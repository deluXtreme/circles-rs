use alloy_primitives::aliases::U192;
use circles_pathfinder::{PathfinderError, create_flow_matrix};

mod common;

#[test]
fn test_create_flow_matrix_simple() {
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    let value = common::wei_from_str(common::ONE_ETH_WEI);

    let transfers = vec![common::sample_transfer_step(
        sender, receiver, sender, value,
    )];

    let result = create_flow_matrix(sender, receiver, value, &transfers);
    assert!(result.is_ok(), "create_flow_matrix should succeed");

    let matrix = result.unwrap();

    // Should have at least sender and receiver in vertices
    assert!(matrix.flow_vertices.len() >= 2);
    assert!(matrix.flow_vertices.contains(&sender));
    assert!(matrix.flow_vertices.contains(&receiver));

    // Should have one flow edge
    assert_eq!(matrix.flow_edges.len(), 1);
    assert_eq!(matrix.flow_edges[0].amount, value);
    assert_eq!(matrix.flow_edges[0].stream_sink_id, 1); // Terminal edge

    // Should have one stream
    assert_eq!(matrix.streams.len(), 1);
}

#[test]
fn test_create_flow_matrix_complex() {
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    let intermediate_a5 = common::addresses::intermediate_a5();
    let intermediate_63 = common::addresses::intermediate_63();
    let token_owner_7b = common::addresses::token_owner_7b();
    let token_owner_f7 = common::addresses::token_owner_f7();

    let value = common::wei_from_str(common::ONE_ETH_WEI);

    let transfers = vec![
        common::sample_transfer_step(sender, intermediate_a5, sender, value),
        common::sample_transfer_step(intermediate_a5, intermediate_63, token_owner_7b, value),
        common::sample_transfer_step(intermediate_63, receiver, token_owner_f7, value),
    ];

    let result = create_flow_matrix(sender, receiver, value, &transfers);
    assert!(result.is_ok(), "create_flow_matrix should succeed");

    let matrix = result.unwrap();

    // Check vertices include all unique addresses
    let expected_addresses = vec![
        sender,
        receiver,
        intermediate_a5,
        intermediate_63,
        token_owner_7b,
        token_owner_f7,
    ];
    assert_eq!(matrix.flow_vertices.len(), expected_addresses.len());

    for addr in expected_addresses {
        assert!(matrix.flow_vertices.contains(&addr));
    }

    // Check flow edges
    assert_eq!(matrix.flow_edges.len(), 3);

    // First two edges should be non-terminal (stream_sink_id = 0)
    assert_eq!(matrix.flow_edges[0].stream_sink_id, 0);
    assert_eq!(matrix.flow_edges[0].amount, value);

    assert_eq!(matrix.flow_edges[1].stream_sink_id, 0);
    assert_eq!(matrix.flow_edges[1].amount, value);

    // Last edge should be terminal (stream_sink_id = 1)
    assert_eq!(matrix.flow_edges[2].stream_sink_id, 1);
    assert_eq!(matrix.flow_edges[2].amount, value);

    // Check streams
    assert_eq!(matrix.streams.len(), 1);
    assert_eq!(matrix.streams[0].flow_edge_ids, vec![2]); // Only terminal edge

    // Check packed coordinates is not empty
    assert!(!matrix.packed_coordinates.is_empty());

    // Each transfer should contribute 3 coordinates (token_owner, from, to)
    // So 3 transfers * 3 coordinates * 2 bytes per coordinate = 18 bytes
    assert_eq!(matrix.packed_coordinates.len(), 18);
}

#[test]
fn test_create_flow_matrix_terminal_sum_mismatch() {
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    let expected_value = common::wei_from_str(common::ONE_ETH_WEI);
    let wrong_value = common::wei_from_str(common::ONE_TENTH_ETH_WEI); // 0.1 ETH

    let transfers = vec![common::sample_transfer_step(
        sender,
        receiver,
        sender,
        wrong_value,
    )];

    let result = create_flow_matrix(sender, receiver, expected_value, &transfers);
    assert!(result.is_err(), "Should fail with mismatched terminal sum");

    match result.unwrap_err() {
        PathfinderError::Imbalanced {
            terminal_sum,
            expected,
        } => {
            assert_eq!(terminal_sum, wrong_value);
            assert_eq!(expected, expected_value);
        }
        other => panic!("Expected Imbalanced error, got: {:?}", other),
    }
}

#[test]
fn test_create_flow_matrix_no_terminal_edges() {
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    let intermediate = common::addresses::intermediate_a5();
    let value = common::wei_from_str(common::ONE_ETH_WEI);

    // Create transfers where none go to the receiver initially
    let transfers = vec![common::sample_transfer_step(
        sender,
        intermediate,
        sender,
        value,
    )];

    let result = create_flow_matrix(sender, receiver, value, &transfers);

    // The function automatically makes the last edge terminal
    // Since the transfer doesn't go to receiver but value matches, it should succeed
    // with the last edge marked as terminal
    assert!(
        result.is_ok(),
        "Should succeed by making last edge terminal"
    );

    let matrix = result.unwrap();
    assert_eq!(matrix.flow_edges.len(), 1);
    assert_eq!(matrix.flow_edges[0].stream_sink_id, 1); // Should be terminal
}

#[test]
fn test_create_flow_matrix_multiple_terminal_edges() {
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    let half_value = common::wei_from_str(common::ONE_ETH_WEI) / U192::from(2);
    let total_value = common::wei_from_str(common::ONE_ETH_WEI);

    // Two transfers to receiver, each with half the value
    let transfers = vec![
        common::sample_transfer_step(sender, receiver, sender, half_value),
        common::sample_transfer_step(sender, receiver, sender, half_value),
    ];

    let result = create_flow_matrix(sender, receiver, total_value, &transfers);
    assert!(
        result.is_ok(),
        "Should succeed with multiple terminal edges"
    );

    let matrix = result.unwrap();

    // Both edges should be terminal
    assert_eq!(matrix.flow_edges.len(), 2);
    assert_eq!(matrix.flow_edges[0].stream_sink_id, 1);
    assert_eq!(matrix.flow_edges[1].stream_sink_id, 1);

    // Stream should reference both terminal edges
    assert_eq!(matrix.streams[0].flow_edge_ids, vec![0, 1]);
}

#[test]
fn test_create_flow_matrix_empty_transfers() {
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    let value = common::wei_from_str(common::ONE_ETH_WEI);

    let transfers = vec![];

    assert!(create_flow_matrix(sender, receiver, value, &transfers).is_err());
}

#[test]
fn test_create_flow_matrix_source_coordinate() {
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    let value = common::wei_from_str(common::ONE_ETH_WEI);

    let transfers = vec![common::sample_transfer_step(
        sender, receiver, sender, value,
    )];

    let result = create_flow_matrix(sender, receiver, value, &transfers);
    assert!(result.is_ok());

    let matrix = result.unwrap();

    // Source coordinate should correspond to sender's position in sorted vertices
    let sender_index = matrix
        .flow_vertices
        .iter()
        .position(|&addr| addr == sender)
        .expect("Sender should be in vertices");

    assert_eq!(matrix.source_coordinate, sender_index as u16);
}
