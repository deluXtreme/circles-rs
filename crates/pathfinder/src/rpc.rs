use crate::PathfinderError;
use alloy_primitives::Address;
use alloy_primitives::aliases::{U192, U256};
use circles_rpc::CirclesRpc;
use circles_types::{FindPathParams, PathfindingResult, PathfindingTransferStep, TransferStep};

pub(crate) fn u256_to_u192(value: U256) -> Result<U192, PathfinderError> {
    let limbs = value.into_limbs();
    if limbs[3] != 0 {
        return Err(PathfinderError::RpcResponse(
            "transfer value exceeds U192".into(),
        ));
    }
    Ok(U192::from_limbs([limbs[0], limbs[1], limbs[2]]))
}

fn convert_step(step: &PathfindingTransferStep) -> Result<TransferStep, PathfinderError> {
    let token_owner: Address = step
        .token_owner
        .parse()
        .map_err(|e| PathfinderError::RpcResponse(format!("invalid tokenOwner: {e}")))?;
    let value = u256_to_u192(step.value)?;
    Ok(TransferStep {
        from_address: step.from,
        to_address: step.to,
        token_owner,
        value,
    })
}

/// Find an optimal path between two addresses in the Circles network.
///
/// This function queries the Circles RPC endpoint to discover a sequence of
/// transfers that can move value from the source to the destination address.
///
/// # Arguments
///
/// * `rpc_url` - The Circles RPC endpoint URL
/// * `from` - Source address for the transfer
/// * `to` - Destination address for the transfer
/// * `target_flow` - Desired amount to transfer
/// * `with_wrap` - Whether to use wrapped token balances
///
/// # Returns
///
/// Returns a vector of `TransferStep` representing the optimal path, or an
/// error if no path exists or the RPC call fails.
///
/// # Examples
///
/// ```text
/// use circles_pathfinder::find_path;
/// use alloy_primitives::{Address, U256};
///
/// async fn example() -> Result<(), Box<dyn std::error::Error>> {
///     let from: Address = "0x1234567890123456789012345678901234567890".parse()?;
///     let to: Address = "0x0987654321098765432109876543210987654321".parse()?;
///     let amount = U256::from(1_000_000_000_000_000_000u64); // 1 ETH in wei
///
///     let transfers = find_path(
///         "https://rpc.aboutcircles.com/",
///         from,
///         to,
///         amount,
///         true // use wrapped balances).await?;
///
///     println!("Found path with {} transfers", transfers.len());
///     Ok(())
///}
/// ```
///
/// # Errors
///
/// - [`PathfinderError::Transport`] - Network/HTTP or underlying client errors
/// - [`PathfinderError::RpcResponse`] - Invalid RPC response or protocol errors
pub async fn find_path(
    rpc_url: &str,
    from: Address,
    to: Address,
    target_flow: U192,
    with_wrap: bool,
) -> Result<Vec<TransferStep>, PathfinderError> {
    let rpc = CirclesRpc::try_from_http(rpc_url)?;
    let params = FindPathParams {
        from,
        to,
        target_flow: U256::from(target_flow),
        use_wrapped_balances: Some(with_wrap),
        from_tokens: None,
        to_tokens: None,
        exclude_from_tokens: None,
        exclude_to_tokens: None,
        simulated_balances: None,
        max_transfers: None,
    };
    let result: PathfindingResult = rpc.pathfinder().find_path(params).await?;
    result
        .transfers
        .iter()
        .map(convert_step)
        .collect::<Result<Vec<_>, _>>()
}

/// Find a path using structured parameters.
pub async fn find_path_with_params(
    rpc_url: &str,
    params: FindPathParams,
) -> Result<Vec<TransferStep>, PathfinderError> {
    let rpc = CirclesRpc::try_from_http(rpc_url)?;
    let result: PathfindingResult = rpc.pathfinder().find_path(params).await?;
    result
        .transfers
        .iter()
        .map(convert_step)
        .collect::<Result<Vec<_>, _>>()
}
