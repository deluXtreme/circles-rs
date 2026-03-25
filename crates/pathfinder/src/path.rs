use crate::PathfinderError;
use alloy_primitives::{Address, I256, U256};
use circles_rpc::CirclesRpc;
use circles_types::{PathfindingResult, TokenInfo};
use circles_utils::converter::atto_static_circles_to_atto_circles;
use std::collections::HashMap;
use std::str::FromStr;

/// Build a map of token info for all token owners the current avatar sends from in the path.
///
/// Normalizes wrapper token types so non-inflationary wrappers are coerced to
/// `CrcV2_ERC20WrapperDeployed_Demurraged` for downstream logic.
pub async fn token_info_map_from_path_via_rpc(
    current_avatar: Address,
    rpc: &CirclesRpc,
    path: &PathfindingResult,
) -> Result<HashMap<Address, TokenInfo>, PathfinderError> {
    let mut unique = Vec::new();
    for t in &path.transfers {
        if t.from == current_avatar
            && let Ok(addr) = Address::from_str(&t.token_owner)
        {
            unique.push(addr);
        }
    }
    unique.sort();
    unique.dedup();
    if unique.is_empty() {
        return Ok(HashMap::new());
    }

    let batch = rpc.token_info().get_token_info_batch(unique).await?;
    let mut map = HashMap::new();
    for mut info in batch {
        let is_wrapper = info.token_type.starts_with("CrcV2_ERC20WrapperDeployed");
        let is_inflationary = info.token_type.contains("Inflationary");
        if is_wrapper && !is_inflationary {
            info.token_type = "CrcV2_ERC20WrapperDeployed_Demurraged".to_string();
        }
        map.insert(info.token, info);
    }
    Ok(map)
}

/// Compatibility wrapper that preserves the existing Rust client-based entrypoint.
pub async fn token_info_map_from_path(
    current_avatar: Address,
    rpc: &CirclesRpc,
    path: &PathfindingResult,
) -> Result<HashMap<Address, TokenInfo>, PathfinderError> {
    token_info_map_from_path_via_rpc(current_avatar, rpc, path).await
}

/// Convenience wrapper for callers that only have an RPC URL.
pub async fn token_info_map_from_path_with_url(
    current_avatar: Address,
    rpc_url: &str,
    path: &PathfindingResult,
) -> Result<HashMap<Address, TokenInfo>, PathfinderError> {
    let rpc = CirclesRpc::try_from_http(rpc_url)?;
    token_info_map_from_path_via_rpc(current_avatar, &rpc, path).await
}

/// Accumulate totals for wrapped tokens present in a path.
///
/// Returns per-wrapper totals and the wrapper's token_type so callers can
/// distinguish inflationary vs demurraged when unwrapping.
pub fn wrapped_totals_from_path(
    path: &PathfindingResult,
    token_info_map: &HashMap<Address, TokenInfo>,
) -> HashMap<Address, (U256, String)> {
    let mut out = HashMap::new();
    for t in &path.transfers {
        if let Ok(owner) = Address::from_str(&t.token_owner)
            && let Some(info) = token_info_map.get(&owner)
            && info.token_type.starts_with("CrcV2_ERC20WrapperDeployed")
        {
            let entry = out
                .entry(owner)
                .or_insert((U256::ZERO, info.token_type.clone()));
            entry.0 = entry.0.saturating_add(t.value);
        }
    }
    out
}

/// Additive parity alias for the TypeScript `getWrappedTokensFromPath` helper.
pub fn get_wrapped_tokens_from_path(
    path: &PathfindingResult,
    token_info_map: &HashMap<Address, TokenInfo>,
) -> HashMap<Address, (U256, String)> {
    wrapped_totals_from_path(path, token_info_map)
}

/// Convert wrapped totals to their underlying avatar/token totals using the current time.
///
/// This mirrors the TypeScript pathfinder helper, which calls the converter
/// with its default "now" semantics for inflationary wrappers.
pub fn expected_unwrapped_totals(
    wrapped_totals: &HashMap<Address, (U256, String)>,
    token_info_map: &HashMap<Address, TokenInfo>,
) -> HashMap<Address, (U256, Address)> {
    expected_unwrapped_totals_at(wrapped_totals, token_info_map, None)
}

/// Convert wrapped totals to their underlying avatar/token totals at an explicit timestamp.
///
/// Use this when you need deterministic conversion behavior instead of the
/// TypeScript-compatible default "now" semantics.
pub fn expected_unwrapped_totals_at(
    wrapped_totals: &HashMap<Address, (U256, String)>,
    token_info_map: &HashMap<Address, TokenInfo>,
    now_unix_seconds: Option<u64>,
) -> HashMap<Address, (U256, Address)> {
    let mut out = HashMap::new();
    for (wrapper, (total, ty)) in wrapped_totals {
        if let Some(info) = token_info_map.get(wrapper) {
            match ty.as_str() {
                "CrcV2_ERC20WrapperDeployed_Demurraged" => {
                    out.insert(*wrapper, (*total, info.token_owner));
                }
                "CrcV2_ERC20WrapperDeployed_Inflationary" => {
                    let amount = atto_static_circles_to_atto_circles(*total, now_unix_seconds);
                    out.insert(*wrapper, (amount, info.token_owner));
                }
                _ => {}
            }
        }
    }
    out
}

/// Replace wrapped token addresses in a path with their underlying avatar tokens.
///
/// Produces a new path suitable for flow matrix construction where token_owner
/// is always the underlying avatar (not the wrapper contract).
pub fn replace_wrapped_tokens_with_avatars(
    path: &PathfindingResult,
    token_info_map: &HashMap<Address, TokenInfo>,
) -> PathfindingResult {
    let transfers = path
        .transfers
        .iter()
        .map(|edge| {
            let token_owner = Address::from_str(&edge.token_owner)
                .ok()
                .and_then(|owner| token_info_map.get(&owner))
                .filter(|info| info.token_type.starts_with("CrcV2_ERC20WrapperDeployed"))
                .map(|info| format!("{:#x}", info.token_owner))
                .unwrap_or_else(|| edge.token_owner.clone());

            circles_types::PathfindingTransferStep {
                token_owner,
                ..edge.clone()
            }
        })
        .collect();

    PathfindingResult {
        max_flow: path.max_flow,
        transfers,
    }
}

/// Replace wrapped token addresses in a path with their underlying avatar tokens.
///
/// Produces a new path suitable for flow matrix construction where token_owner
/// is always the underlying avatar (not the wrapper contract).
pub fn replace_wrapped_tokens(
    path: &PathfindingResult,
    unwrapped: &HashMap<Address, (U256, Address)>,
) -> PathfindingResult {
    let mut wrapper_to_avatar = HashMap::new();
    for (wrapper, (_, avatar)) in unwrapped {
        wrapper_to_avatar.insert(wrapper.to_string().to_lowercase(), *avatar);
    }

    let transfers = path
        .transfers
        .iter()
        .map(|edge| {
            let token_owner = wrapper_to_avatar
                .get(&edge.token_owner.to_lowercase())
                .map(|avatar| format!("{avatar:#x}"))
                .unwrap_or_else(|| edge.token_owner.clone());
            circles_types::PathfindingTransferStep {
                token_owner,
                ..edge.clone()
            }
        })
        .collect();

    PathfindingResult {
        max_flow: path.max_flow,
        transfers,
    }
}

/// Scale down all transfer values by retain_bps (1e12 basis).
///
/// Useful for netting checks: shrink a path to match a reduced payment amount
/// while preserving proportions.
pub fn shrink_path_values(
    path: &PathfindingResult,
    sink: Address,
    retain_bps: U256,
) -> PathfindingResult {
    let denom = U256::from(1_000_000_000_000u64);
    let mut incoming_to_sink: HashMap<Address, U256> = HashMap::new();
    let transfers = path
        .transfers
        .iter()
        .filter_map(|edge| {
            let scaled = edge.value.saturating_mul(retain_bps) / denom;
            if scaled.is_zero() {
                return None;
            }
            let mut next = edge.clone();
            next.value = scaled;
            *incoming_to_sink.entry(edge.to).or_insert(U256::ZERO) += scaled;
            Some(next)
        })
        .collect::<Vec<_>>();

    let max_flow = incoming_to_sink.get(&sink).copied().unwrap_or_default();

    PathfindingResult {
        max_flow,
        transfers,
    }
}

/// Compute netted flow per address (sink positive, source negative).
pub fn compute_netted_flow(path: &PathfindingResult) -> HashMap<Address, I256> {
    let mut net = HashMap::new();
    for t in &path.transfers {
        let amount: I256 = I256::from_raw(t.value);
        net.entry(t.from)
            .and_modify(|v| *v -= amount)
            .or_insert(-amount);
        net.entry(t.to)
            .and_modify(|v| *v += amount)
            .or_insert(amount);
    }
    net
}

/// Assert that source/sink/intermediate balances match expected netting rules.
///
/// - Source must be net negative, sink net positive, intermediates balanced.
/// - If source == sink, all vertices must net to zero.
pub fn assert_no_netted_flow_mismatch(
    path: &PathfindingResult,
    override_source: Option<Address>,
    override_sink: Option<Address>,
) -> Result<(), PathfinderError> {
    let net = compute_netted_flow(path);
    let (source, sink) = get_source_and_sink(path, override_source, override_sink)?;
    let endpoints_coincide = source == sink;

    for (addr, balance) in net {
        if endpoints_coincide {
            if balance != I256::ZERO {
                return Err(PathfinderError::RpcResponse(format!(
                    "Vertex {addr:#x} is unbalanced: {balance}"
                )));
            }
            continue;
        }

        let is_source = addr == source;
        let is_sink = addr == sink;
        if is_source && balance >= I256::ZERO {
            return Err(PathfinderError::RpcResponse(format!(
                "Source {addr:#x} should be net negative, got {balance}"
            )));
        }
        if is_sink && balance <= I256::ZERO {
            return Err(PathfinderError::RpcResponse(format!(
                "Sink {addr:#x} should be net positive, got {balance}"
            )));
        }
        if !is_source && !is_sink && balance != I256::ZERO {
            return Err(PathfinderError::RpcResponse(format!(
                "Vertex {addr:#x} is unbalanced: {balance}"
            )));
        }
    }
    Ok(())
}

fn get_source_and_sink(
    path: &PathfindingResult,
    override_source: Option<Address>,
    override_sink: Option<Address>,
) -> Result<(Address, Address), PathfinderError> {
    use std::collections::HashSet;

    let mut senders = Vec::new();
    let mut receivers = Vec::new();
    let mut sender_set = HashSet::new();
    let mut receiver_set = HashSet::new();

    for transfer in &path.transfers {
        if sender_set.insert(transfer.from) {
            senders.push(transfer.from);
        }
        if receiver_set.insert(transfer.to) {
            receivers.push(transfer.to);
        }
    }

    let source = senders
        .iter()
        .find(|a| !receiver_set.contains(*a))
        .copied()
        .or(override_source);
    let sink = receivers
        .iter()
        .find(|a| !sender_set.contains(*a))
        .copied()
        .or(override_sink);

    match (source, sink) {
        (Some(s), Some(t)) => Ok((s, t)),
        _ => Err(PathfinderError::RpcResponse(
            "Could not determine unique source/sink".into(),
        )),
    }
}
