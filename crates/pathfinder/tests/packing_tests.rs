use alloy_primitives::aliases::U192;
use circles_pathfinder::{pack_coordinates, transform_to_flow_vertices};

mod common;

#[test]
fn test_pack_coordinates() {
    let coords = vec![0x1234, 0x5678];
    let packed = pack_coordinates(&coords);

    // pack_coordinates returns Vec<u8>, not a hex string
    // Each u16 becomes 2 bytes in big-endian format
    let expected: Vec<u8> = vec![0x12, 0x34, 0x56, 0x78];
    assert_eq!(packed, expected);
}

#[test]
fn test_pack_coordinates_empty() {
    let coords = vec![];
    let packed = pack_coordinates(&coords);
    assert_eq!(packed, Vec::<u8>::new());
}

#[test]
fn test_pack_coordinates_single() {
    let coords = vec![0xabcd];
    let packed = pack_coordinates(&coords);
    let expected: Vec<u8> = vec![0xab, 0xcd];
    assert_eq!(packed, expected);
}

#[test]
fn test_transform_to_flow_vertices() {
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    let intermediate = common::addresses::intermediate_a5();

    let transfers = vec![common::sample_transfer_step(
        sender,
        intermediate,
        sender,
        U192::from(1000),
    )];

    let (sorted_vertices, idx) = transform_to_flow_vertices(&transfers, sender, receiver);

    // Should include sender, receiver, and all addresses from transfers
    assert!(sorted_vertices.len() >= 3);
    assert!(idx.len() >= 3);

    // Check that all required addresses are present
    assert!(sorted_vertices.contains(&sender));
    assert!(sorted_vertices.contains(&receiver));
    assert!(sorted_vertices.contains(&intermediate));

    // Check that index map has entries for all vertices
    for vertex in &sorted_vertices {
        assert!(idx.contains_key(vertex));
    }

    // Check that indices are valid
    for (address, index) in &idx {
        assert!(*index < sorted_vertices.len());
        assert_eq!(sorted_vertices[*index], *address);
    }
}

#[test]
fn test_transform_to_flow_vertices_complex() {
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

    let (sorted_vertices, idx) = transform_to_flow_vertices(&transfers, sender, receiver);

    // Should include all unique addresses
    let expected_addresses = vec![
        sender,
        receiver,
        intermediate_a5,
        intermediate_63,
        token_owner_7b,
        token_owner_f7,
    ];

    assert_eq!(sorted_vertices.len(), expected_addresses.len());

    // All expected addresses should be present
    for addr in expected_addresses {
        assert!(sorted_vertices.contains(&addr));
        assert!(idx.contains_key(&addr));
    }

    // Vertices should be sorted by byte representation
    for i in 1..sorted_vertices.len() {
        assert!(sorted_vertices[i - 1].as_slice() <= sorted_vertices[i].as_slice());
    }
}

#[test]
fn test_transform_to_flow_vertices_duplicate_addresses() {
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    let value = U192::from(1000);

    // Create transfers where the same address appears multiple times
    let transfers = vec![
        common::sample_transfer_step(sender, receiver, sender, value),
        common::sample_transfer_step(sender, receiver, sender, value), // Duplicate
    ];

    let (sorted_vertices, idx) = transform_to_flow_vertices(&transfers, sender, receiver);

    // Should only have unique addresses
    assert_eq!(sorted_vertices.len(), 2); // sender and receiver only
    assert_eq!(idx.len(), 2);

    assert!(sorted_vertices.contains(&sender));
    assert!(sorted_vertices.contains(&receiver));
}

#[test]
fn test_transform_to_flow_vertices_sender_receiver_same() {
    let address = common::addresses::sender();
    let value = U192::from(1000);

    let transfers = vec![common::sample_transfer_step(
        address, address, address, value,
    )];

    let (sorted_vertices, idx) = transform_to_flow_vertices(&transfers, address, address);

    // Should only have one unique address
    assert_eq!(sorted_vertices.len(), 1);
    assert_eq!(idx.len(), 1);
    assert_eq!(sorted_vertices[0], address);
    assert_eq!(idx[&address], 0);
}
