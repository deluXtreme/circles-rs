use crate::find_path_with_params;
use crate::{PathfinderError, hub::PathData, rpc::FindPathParams};
use alloy_primitives::aliases::U192;
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::Address;

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
}
