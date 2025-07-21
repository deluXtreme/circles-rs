use crate::{FlowEdge, PathData, Stream};
use crate::{FlowMatrix, find_path_with_params};
use crate::{PathfinderError, rpc::FindPathParams};
use alloy_primitives::Address;
use alloy_primitives::aliases::{U192, U256};
use alloy_sol_types::SolValue;
use circles_types::TransferStep;

/// High-level function that combines pathfinding and matrix creation
///
/// This function performs the complete flow from pathfinding to contract-ready
/// matrix creation in a single call. It automatically handles the case where
/// the available flow might be less than the requested flow.
///
/// # Arguments
/// * `rpc_url` - The RPC endpoint URL
/// * `params` - Path finding parameters
///
/// # Returns
/// A `PathData` with types ready for smart contract calls
///
/// # Example
/// ```rust,no_run
/// use circles_pathfinder::{FindPathParams, prepare_flow_for_contract};
/// use alloy_primitives::{Address, aliases::U192};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let params = FindPathParams {
///     from: "0x123...".parse()?,
///     to: "0x456...".parse()?,
///     target_flow: U192::from(1000000000000000000u64), // 1 CRC in wei
///     use_wrapped_balances: Some(true),
///     from_tokens: None,
///     to_tokens: None,
///     exclude_from_tokens: None,
///     exclude_to_tokens: None,
/// };
///
/// let path_data = prepare_flow_for_contract("https://rpc.example.com", params).await?;
///
/// // Ready to use with smart contract calls
/// let (vertices, edges, streams, coords) = path_data.to_contract_params();
/// // contract.some_function(vertices, edges, streams, coords);
/// # Ok(())
/// # }
/// ```
pub async fn prepare_flow_for_contract(
    rpc_url: &str,
    params: FindPathParams,
) -> Result<PathData, PathfinderError> {
    // Step 1: Find the path
    let transfers = find_path_with_params(rpc_url, params.clone()).await?;

    // Step 2: Create PathData from transfers (handles flow calculation internally)
    PathData::from_transfers(&transfers, params.from, params.to, params.target_flow)
}

/// Prepare flow for contract using individual parameters (legacy compatibility)
///
/// This is a convenience wrapper around `prepare_flow_for_contract` for users
/// who prefer to pass individual parameters instead of a struct.
pub async fn prepare_flow_for_contract_simple(
    rpc_url: &str,
    from: alloy_primitives::Address,
    to: alloy_primitives::Address,
    target_flow: U192,
    use_wrapped_balances: bool,
) -> Result<PathData, PathfinderError> {
    let params = FindPathParams {
        from,
        to,
        target_flow,
        use_wrapped_balances: Some(use_wrapped_balances),
        from_tokens: None,
        to_tokens: None,
        exclude_from_tokens: None,
        exclude_to_tokens: None,
    };

    prepare_flow_for_contract(rpc_url, params).await
}

/// Get the maximum available flow between two addresses
///
/// This function finds a path and returns the maximum amount that can actually
/// be transferred, which might be less than the requested amount due to
/// liquidity constraints.
///
/// # Returns
/// A tuple of (available_flow, transfers) where available_flow is the actual
/// amount that can be transferred.
pub async fn get_available_flow(
    rpc_url: &str,
    params: FindPathParams,
) -> Result<(U192, Vec<TransferStep>), PathfinderError> {
    let transfers = find_path_with_params(rpc_url, params.clone()).await?;

    let available_flow: U192 = transfers
        .iter()
        .filter(|t| t.to_address == params.to)
        .map(|t| t.value)
        .sum();

    Ok((available_flow, transfers))
}

pub fn encode_redeem_trusted_data(
    flow_vertices: Vec<Address>,
    flow: Vec<FlowEdge>,
    streams: Vec<Stream>,
    packed_coordinates: Vec<u8>,
    source_coordinate: U256,
) -> Vec<u8> {
    (
        flow_vertices,
        flow,
        streams,
        packed_coordinates,
        source_coordinate,
    )
        .abi_encode_params()
}

pub fn encode_redeem_flow_matrix(matrix: FlowMatrix) -> Vec<u8> {
    encode_redeem_trusted_data(
        matrix.flow_vertices,
        matrix.flow_edges,
        matrix.streams,
        matrix.packed_coordinates,
        matrix.source_coordinate,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::Address;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_prepare_flow_for_contract_simple() {
        let sender = Address::ZERO;
        let receiver = Address::from([1u8; 20]);
        let value = U192::from(1000u64);

        // This will fail with network error in tests, but tests the API
        let result = prepare_flow_for_contract_simple(
            "http://invalid-rpc.com",
            sender,
            receiver,
            value,
            true,
        )
        .await;

        // Should fail with RPC error, not panic
        assert!(result.is_err());
    }

    #[test]
    fn test_find_path_params_creation() {
        let params = FindPathParams {
            from: Address::ZERO,
            to: Address::from([1u8; 20]),
            target_flow: U192::from(1000u64),
            use_wrapped_balances: Some(true),
            from_tokens: None,
            to_tokens: None,
            exclude_from_tokens: None,
            exclude_to_tokens: None,
        };

        assert_eq!(params.from, Address::ZERO);
        assert_eq!(params.to, Address::from([1u8; 20]));
        assert_eq!(params.target_flow, U192::from(1000u64));
        assert_eq!(params.use_wrapped_balances, Some(true));
    }

    #[tokio::test]
    async fn test_redeem_trusted_data_encoding() {
        let rpc_url = "https://rpc.aboutcircles.com/";

        // Hardcoded JSON payloads as strings
        let _payload1 = r#"
        {
            "id": "0x4652021487668a2c25747c81dc7d553d3c3121df19fac8c7f49e5adc478d1d31",
            "subscriber": "0xcf6dc192dc292d5f2789da2db02d6dd4f41f4214",
            "recipient": "0x6b69683c8897e3d18e74b1ba117b49f80423da5d",
            "amount": "10000000000000000",
            "category": "trusted",
            "next_redeem_at": 0
        }
        "#;

        let _payload2 = r#"
        {
            "id": "0xdc849e3b51c6cd3b3c5b5f028c7889f1b2d722f9f8ddbaffd3693208e34a494e",
            "subscriber": "0x6b69683c8897e3d18e74b1ba117b49f80423da5d",
            "recipient": "0xcf6dc192dc292d5f2789da2db02d6dd4f41f4214",
            "amount": "10000000000000000",
            "category": "trusted",
            "next_redeem_at": 0
        }
        "#;

        // Parse payloads (assuming we have serde for real test)
        // For simplicity, hardcode values
        let subs = vec![
            (
                "0x4652021487668a2c25747c81dc7d553d3c3121df19fac8c7f49e5adc478d1d31",
                "0xcf6dc192dc292d5f2789da2db02d6dd4f41f4214",
                "0x6b69683c8897e3d18e74b1ba117b49f80423da5d",
                "10000000000000000",
            ),
            (
                "0xdc849e3b51c6cd3b3c5b5f028c7889f1b2d722f9f8ddbaffd3693208e34a494e",
                "0x6b69683c8897e3d18e74b1ba117b49f80423da5d",
                "0xcf6dc192dc292d5f2789da2db02d6dd4f41f4214",
                "10000000000000000",
            ),
        ];

        for (_id, sub_str, rec_str, amt_str) in subs {
            let subscriber = Address::from_str(sub_str).unwrap();
            let recipient = Address::from_str(rec_str).unwrap();
            let amount = U192::from_str_radix(amt_str, 10).unwrap();

            let path_data = prepare_flow_for_contract_simple(
                rpc_url, subscriber, recipient, amount, false, // use_wrapped_balances = false
            )
            .await
            .expect("Failed to prepare flow data");

            let data = encode_redeem_trusted_data(
                path_data.flow_vertices,
                path_data.flow_edges,
                path_data.streams,
                path_data.packed_coordinates,
                path_data.source_coordinate,
            );
            println!("Encoded data: {data:?}");
            assert!(!data.is_empty(), "Encoded data should not be empty");
        }
    }
}
