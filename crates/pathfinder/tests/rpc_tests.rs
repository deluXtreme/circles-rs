use alloy_primitives::aliases::U192;
use circles_pathfinder::{PathfinderError, find_path};

mod common;

#[tokio::test]
async fn test_find_path() {
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();

    // Convert 1 ETH to wei (1e18)
    let value = common::wei_from_str(common::ONE_ETH_WEI);

    let result = find_path(
        common::CIRCLES_RPC,
        sender,
        receiver,
        value,
        true, // with_wrap = true (equivalent to use_wrapped_balances)
    )
    .await;

    // Note: The original test just logs the result, but you might want to add assertions
    // based on your specific requirements
    assert!(result.is_ok(), "find_path should not return an error");

    // Optional: Add more specific assertions
    if let Ok(transfers) = result {
        assert!(
            !transfers.is_empty(),
            "Should return at least one transfer step"
        );
        // You could add more assertions here based on expected behavior
    }
}

#[tokio::test]
async fn test_find_path_with_invalid_rpc() {
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    let value = common::wei_from_str(common::ONE_ETH_WEI);

    let result = find_path("http://invalid-rpc-url.com", sender, receiver, value, true).await;

    assert!(result.is_err(), "Should return error for invalid RPC URL");

    // Check that it's the right kind of error
    match result.unwrap_err() {
        PathfinderError::Rpc(_) => {} // Expected
        other => panic!("Expected RPC error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_find_path_with_zero_value() {
    let sender = common::addresses::sender();
    let receiver = common::addresses::receiver();
    let value = U192::ZERO;

    let result = find_path(common::CIRCLES_RPC, sender, receiver, value, true).await;

    // This test checks edge case behavior - the result depends on your RPC implementation
    // You might expect this to succeed with empty transfers or fail with a specific error
    match result {
        Ok(transfers) => {
            // If it succeeds, transfers might be empty
            println!(
                "Zero value request succeeded with {} transfers",
                transfers.len()
            );
        }
        Err(e) => {
            // If it fails, it should be a meaningful error
            println!("Zero value request failed as expected: {}", e);
        }
    }
}

// Note: We don't test JsonRpcResp parsing directly since it's a private implementation detail.
// The parsing is tested indirectly through the find_path integration tests.

#[tokio::test]
async fn test_find_path_same_sender_receiver() {
    let address = common::addresses::sender();
    let value = common::wei_from_str(common::ONE_ETH_WEI);

    let result = find_path(
        common::CIRCLES_RPC,
        address,
        address, // Same address for sender and receiver
        value,
        true,
    )
    .await;

    // This tests edge case behavior when sender == receiver
    // The result depends on your RPC implementation
    match result {
        Ok(transfers) => {
            println!(
                "Same address request succeeded with {} transfers",
                transfers.len()
            );
        }
        Err(e) => {
            println!("Same address request failed: {}", e);
        }
    }
}
