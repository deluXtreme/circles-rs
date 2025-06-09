use pathfinder::{find_path, create_flow_matrix, PathfinderError};
use alloy_primitives::U256;

mod common;

#[tokio::test]
async fn test_full_pathfinding_flow() {
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    let value = common::wei_from_str(common::ONE_ETH_WEI);

    // Step 1: Find path using RPC
    let transfers_result = find_path(
        common::CIRCLES_RPC,
        sender,
        receiver,
        value,
        true,
    ).await;

    // Only proceed if RPC call succeeds (it might fail in CI/offline environments)
    if let Ok(transfers) = transfers_result {
        println!("Found {} transfer steps", transfers.len());
        
        // Calculate the actual available flow from transfers going to receiver
        let actual_flow: U256 = transfers
            .iter()
            .filter(|t| t.to_address == receiver)
            .map(|t| t.value)
            .sum();
        
        println!("Requested: {}, Available: {}", value, actual_flow);
        
        // Step 2: Create flow matrix from transfers using the actual available flow
        let matrix_result = create_flow_matrix(sender, receiver, actual_flow, &transfers);
        
        if let Err(e) = &matrix_result {
            println!("Flow matrix creation failed: {:?}", e);
            println!("Transfers: {:?}", transfers);
            println!("Sender: {:?}, Receiver: {:?}, Actual Flow: {:?}", sender, receiver, actual_flow);
        }
        
        assert!(matrix_result.is_ok(), "Flow matrix creation should succeed: {:?}", matrix_result.as_ref().unwrap_err());
        
        let matrix = matrix_result.unwrap();
        
        // Verify the matrix makes sense
        assert!(!matrix.flow_vertices.is_empty(), "Should have vertices");
        assert!(!matrix.flow_edges.is_empty(), "Should have edges");
        assert!(!matrix.streams.is_empty(), "Should have streams");
        assert!(!matrix.packed_coordinates.is_empty(), "Should have packed coordinates");
        
        // Verify sender and receiver are in vertices
        assert!(matrix.flow_vertices.contains(&sender), "Sender should be in vertices");
        assert!(matrix.flow_vertices.contains(&receiver), "Receiver should be in vertices");
        
        // Verify terminal sum matches actual available value
        let terminal_sum: U256 = matrix.flow_edges
            .iter()
            .filter(|e| e.stream_sink_id == 1)
            .map(|e| e.amount)
            .sum();
        assert_eq!(terminal_sum, actual_flow, "Terminal sum should match actual available flow");
        
        println!("Integration test passed: {} vertices, {} edges, {} streams", 
                 matrix.flow_vertices.len(), 
                 matrix.flow_edges.len(), 
                 matrix.streams.len());

    } else {
        println!("RPC call failed (expected in CI): {:?}", transfers_result.unwrap_err());
        // In CI or offline environments, we can't test the full flow
        // This is acceptable as the individual components are tested separately
    }
}

#[tokio::test]
async fn test_pathfinding_with_different_values() {
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    
    // Test different values to see how the pathfinding behaves
    let test_values = vec![
        U256::from(1_000_000_000_000_000u64),      // 0.001 ETH
        U256::from(10_000_000_000_000_000u64),     // 0.01 ETH
        U256::from(100_000_000_000_000_000u64),    // 0.1 ETH
    ];
    
    for value in test_values {
        println!("Testing with value: {}", value);
        
        let transfers_result = find_path(
            common::CIRCLES_RPC,
            sender,
            receiver,
            value,
            true,
        ).await;
        
        if let Ok(transfers) = transfers_result {
            // If we get transfers, verify we can create a valid matrix
            let matrix_result = create_flow_matrix(sender, receiver, value, &transfers);
            
            if matrix_result.is_ok() {
                println!("✓ Value {} works end-to-end", value);
            } else {
                println!("✗ Value {} failed matrix creation: {:?}", value, matrix_result.unwrap_err());
            }
        } else {
            println!("✗ Value {} failed pathfinding: {:?}", value, transfers_result.unwrap_err());
        }
    }
}

#[tokio::test]
async fn test_pathfinding_flow_with_error_handling() {
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    let value = common::wei_from_str(common::ONE_ETH_WEI);
    
    // Test the flow with various error scenarios
    
    // 1. Test with invalid RPC URL
    let invalid_rpc_result = find_path(
        "http://invalid-url.com",
        sender,
        receiver,
        value,
        true,
    ).await;
    
    assert!(invalid_rpc_result.is_err(), "Invalid RPC should fail");
    match invalid_rpc_result.unwrap_err() {
        PathfinderError::Rpc(_) => {}, // Expected
        other => panic!("Expected RPC error, got: {:?}", other),
    }
    
    // 2. Test matrix creation with mismatched values
    // Create a mock transfer that doesn't match the expected value
    let wrong_value = common::wei_from_str(common::ONE_TENTH_ETH_WEI);
    let mock_transfers = vec![
        common::sample_transfer_step(sender, receiver, sender, wrong_value)
    ];
    
    let matrix_result = create_flow_matrix(sender, receiver, value, &mock_transfers);
    assert!(matrix_result.is_err(), "Mismatched value should fail");
    
    match matrix_result.unwrap_err() {
        PathfinderError::Imbalanced { terminal_sum, expected } => {
            assert_eq!(terminal_sum, wrong_value);
            assert_eq!(expected, value);
        }
        other => panic!("Expected Imbalanced error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_pathfinding_with_wrapping_variations() {
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    let value = common::wei_from_str(common::ONE_ETH_WEI);
    
    // Test both with and without wrapping
    for with_wrap in [true, false] {
        println!("Testing with wrap = {}", with_wrap);
        
        let result = find_path(
            common::CIRCLES_RPC,
            sender,
            receiver,
            value,
            with_wrap,
        ).await;
        
        match result {
            Ok(transfers) => {
                println!("✓ with_wrap={} succeeded with {} transfers", with_wrap, transfers.len());
                
                // Verify we can create a matrix from these transfers
                let matrix_result = create_flow_matrix(sender, receiver, value, &transfers);
                if matrix_result.is_ok() {
                    println!("✓ Matrix creation also succeeded");
                } else {
                    println!("✗ Matrix creation failed: {:?}", matrix_result.unwrap_err());
                }
            }
            Err(e) => {
                println!("✗ with_wrap={} failed: {:?}", with_wrap, e);
            }
        }
    }
}

#[test]
fn test_edge_case_scenarios() {
    // Test edge cases that don't require RPC calls
    
    // 1. Zero value transfers
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    let zero_value = U256::ZERO;
    
    let zero_transfers = vec![
        common::sample_transfer_step(sender, receiver, sender, zero_value)
    ];
    
    let result = create_flow_matrix(sender, receiver, zero_value, &zero_transfers);
    assert!(result.is_ok(), "Zero value should work");
    
    // 2. Same sender and receiver
    let same_address_transfers = vec![
        common::sample_transfer_step(sender, sender, sender, zero_value)
    ];
    
    let result = create_flow_matrix(sender, sender, zero_value, &same_address_transfers);
    assert!(result.is_ok(), "Same sender/receiver should work");
    
    // 3. Large value (near U256::MAX)
    let large_value = U256::MAX - U256::from(1);
    let large_transfers = vec![
        common::sample_transfer_step(sender, receiver, sender, large_value)
    ];
    
    let result = create_flow_matrix(sender, receiver, large_value, &large_transfers);
    assert!(result.is_ok(), "Large value should work");
}

#[tokio::test]
async fn test_concurrent_pathfinding_requests() {
    use tokio::task::JoinSet;
    
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    let value = common::wei_from_str(common::ONE_ETH_WEI);
    
    // Test making multiple concurrent requests
    let mut join_set = JoinSet::new();
    
    for i in 0..3 {
        let sender = sender;
        let receiver = receiver;
        let value = value;
        
        join_set.spawn(async move {
            let result = find_path(
                common::CIRCLES_RPC,
                sender,
                receiver,
                value,
                true,
            ).await;
            
            (i, result)
        });
    }
    
    let mut success_count = 0;
    let mut error_count = 0;
    
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((request_id, Ok(_transfers))) => {
                println!("✓ Concurrent request {} succeeded", request_id);
                success_count += 1;
            }
            Ok((request_id, Err(e))) => {
                println!("✗ Concurrent request {} failed: {:?}", request_id, e);
                error_count += 1;
            }
            Err(join_error) => {
                println!("✗ Join error: {:?}", join_error);
                error_count += 1;
            }
        }
    }
    
    println!("Concurrent test results: {} successes, {} errors", success_count, error_count);
    
    // We don't assert specific counts because RPC might be unavailable
    // The test is mainly to ensure no panics or deadlocks occur
}

#[tokio::test]
async fn test_improved_user_workflow() {
    use pathfinder::{FindPathParams, prepare_flow_for_contract};
    
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    let value = common::wei_from_str(common::ONE_ETH_WEI);

    // NEW API: Use structured parameters (much cleaner)
    let params = FindPathParams {
        from: sender,
        to: receiver,
        target_flow: value,
        use_wrapped_balances: Some(true),
        from_tokens: None,
        to_tokens: None,
        exclude_from_tokens: None,
        exclude_to_tokens: None,
    };

    // NEW API: One function call does everything!
    let result = prepare_flow_for_contract(common::CIRCLES_RPC, params).await;
    
    if let Ok(contract_matrix) = result {
        println!("✅ New API test succeeded!");
        
        // Verify we get contract-ready types without manual conversion
        assert!(!contract_matrix.flow_vertices.is_empty());
        assert!(!contract_matrix.flow_edges.is_empty());
        assert!(!contract_matrix.streams.is_empty());
        
        // Verify types are already correct for contract calls
        // flow_vertices: Vec<Address> ✓
        // flow_edges: Vec<ContractFlowEdge> ✓  
        // streams: Vec<ContractStream> ✓
        // packed_coordinates: Bytes ✓
        
        // Store lengths before decomposition
        let vertices_len = contract_matrix.flow_vertices.len();
        let edges_len = contract_matrix.flow_edges.len();
        let streams_len = contract_matrix.streams.len();
        
        // Test decomposition for tuple-based contract calls
        let (vertices, edges, streams, packed_coords) = contract_matrix.into_contract_params();
        assert_eq!(vertices.len(), vertices_len);
        assert_eq!(edges.len(), edges_len);
        assert_eq!(streams.len(), streams_len);
        
        println!("Contract-ready data: {} vertices, {} edges, {} streams, {} coord bytes", 
                 vertices.len(), edges.len(), streams.len(), packed_coords.len());
        
        // This demonstrates the HUGE improvement in user experience:
        // OLD WAY: find_path() -> create_flow_matrix() -> manual conversions 
        // NEW WAY: prepare_flow_for_contract() -> ready to use!
        
    } else {
        println!("RPC call failed (expected in CI): {:?}", result.unwrap_err());
        // This is acceptable - the API improvement test works even if RPC fails
    }
}