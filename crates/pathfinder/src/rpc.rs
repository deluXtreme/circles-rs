use crate::PathfinderError;
use alloy_primitives::Address;
use alloy_primitives::aliases::U192;
use circles_types::TransferStep;
use serde_json::json;

/// Parameters for pathfinding operations.
///
/// This struct provides a clean way to pass pathfinding parameters,
/// with optional fields for advanced filtering and routing control.
///
/// # Examples
///
/// ```rust
/// use circles_pathfinder::FindPathParams;
/// use alloy_primitives::{Address, aliases::U192};
///
/// let params = FindPathParams {
///     from: "0xC3CCd9455b301D01d69DFB0b9Fc38Bee39829598".parse()?,
///     to: "0xf48554937f18885c7f15c432c596b5843648231D".parse()?,
///     target_flow: U192::from(1000u64),
///     use_wrapped_balances: Some(true),
///     // Optional filters
///     from_tokens: None,
///     to_tokens: None,
///     exclude_from_tokens: None,
///     exclude_to_tokens: None,
/// };
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct FindPathParams {
    /// Source address
    pub from: Address,
    /// Destination address
    pub to: Address,
    /// Target flow amount
    pub target_flow: U192,
    /// Whether to use wrapped balances
    pub use_wrapped_balances: Option<bool>,
    /// Specific tokens to use from the source (optional)
    pub from_tokens: Option<Vec<Address>>,
    /// Specific tokens to accept at destination (optional)
    pub to_tokens: Option<Vec<Address>>,
    /// Tokens to exclude from source (optional)
    pub exclude_from_tokens: Option<Vec<Address>>,
    /// Tokens to exclude at destination (optional)
    pub exclude_to_tokens: Option<Vec<Address>>,
}

#[derive(serde::Deserialize, Debug)]
struct JsonRpcResp {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u32,
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

impl TryFrom<JsonRpcResp> for Vec<TransferStep> {
    type Error = PathfinderError;

    fn try_from(resp: JsonRpcResp) -> Result<Self, Self::Error> {
        if let Some(err) = resp.error {
            return Err(PathfinderError::JsonRpc(err.to_string()));
        }

        let transfers = resp
            .result
            .ok_or_else(|| PathfinderError::JsonRpc("missing result".into()))?
            .get("transfers")
            .ok_or_else(|| PathfinderError::JsonRpc("missing transfers".into()))?
            .as_array()
            .ok_or_else(|| PathfinderError::JsonRpc("transfers not array".into()))?
            .iter()
            .map(|t| -> Result<TransferStep, PathfinderError> {
                let from_str = t["from"]
                    .as_str()
                    .ok_or_else(|| PathfinderError::JsonRpc("from field is not a string".into()))?;
                let to_str = t["to"]
                    .as_str()
                    .ok_or_else(|| PathfinderError::JsonRpc("to field is not a string".into()))?;
                let token_owner_str = t["tokenOwner"].as_str().ok_or_else(|| {
                    PathfinderError::JsonRpc("tokenOwner field is not a string".into())
                })?;
                let value_str = t["value"].as_str().ok_or_else(|| {
                    PathfinderError::JsonRpc("value field is not a string".into())
                })?;

                let from_address = from_str.parse::<Address>().map_err(|e| {
                    PathfinderError::JsonRpc(format!(
                        "failed to parse from address '{from_str}': {e}"
                    ))
                })?;
                let to_address = to_str.parse::<Address>().map_err(|e| {
                    PathfinderError::JsonRpc(format!("failed to parse to address '{to_str}': {e}"))
                })?;
                let token_owner = token_owner_str.parse::<Address>().map_err(|e| {
                    PathfinderError::JsonRpc(format!(
                        "failed to parse token_owner address '{token_owner_str}': {e}"
                    ))
                })?;

                let value_u128 = value_str.parse::<u128>().map_err(|e| {
                    PathfinderError::JsonRpc(format!("failed to parse value '{value_str}': {e}"))
                })?;

                Ok(TransferStep {
                    from_address,
                    to_address,
                    token_owner,
                    value: U192::from(value_u128),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(transfers)
    }
}

/// Find a path using structured parameters
///
/// This is a convenience function that takes a `FindPathParams` struct
/// instead of individual parameters. Currently only implements the basic
/// functionality (from, to, target_flow, use_wrapped_balances).
///
/// # Note
/// Additional filtering parameters (from_tokens, to_tokens, etc.) are not yet
/// implemented in the underlying RPC call but are included in the struct for
/// future compatibility.
pub async fn find_path_with_params(
    rpc_url: &str,
    params: FindPathParams,
) -> Result<Vec<TransferStep>, PathfinderError> {
    find_path(
        rpc_url,
        params.from,
        params.to,
        params.target_flow,
        params.use_wrapped_balances.unwrap_or(false),
    )
    .await
    // TODO: Implement support for additional parameters:
    // - from_tokens
    // - to_tokens
    // - exclude_from_tokens
    // - exclude_to_tokens
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
/// - [`PathfinderError::Rpc`] - Network or HTTP errors
/// - [`PathfinderError::JsonRpc`] - Invalid RPC response or protocol errors
pub async fn find_path(
    rpc_url: &str,
    from: Address,
    to: Address,
    target_flow: U192,
    with_wrap: bool,
) -> Result<Vec<TransferStep>, PathfinderError> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "circlesV2_findPath",
        "params": [{
            "Source": format!("{:#x}", from),
            "Sink": format!("{:#x}", to),
            "TargetFlow": target_flow.to_string(),
            "WithWrap": with_wrap,
        }]
    });

    let resp: JsonRpcResp = reqwest::Client::new()
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    if let Some(err) = resp.error {
        return Err(PathfinderError::JsonRpc(err.to_string()));
    }

    let transfers = resp.try_into()?;

    Ok(transfers)
}
