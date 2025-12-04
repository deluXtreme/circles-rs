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
pub async fn token_info_map_from_path(
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

/// Convert wrapped totals to their underlying avatar/token totals.
///
/// Inflationary wrappers are converted back to attoCircles using the converter
/// and the token's timestamp hint; demurraged wrappers are currently identity.
pub fn expected_unwrapped_totals(
    wrapped_totals: &HashMap<Address, (U256, String)>,
    token_info_map: &HashMap<Address, TokenInfo>,
) -> HashMap<Address, (U256, Address)> {
    let mut out = HashMap::new();
    for (wrapper, (total, ty)) in wrapped_totals {
        if let Some(info) = token_info_map.get(wrapper) {
            let ts_hint = if info.timestamp > 0 {
                Some(info.timestamp)
            } else {
                None
            };
            let amount = if ty == "CrcV2_ERC20WrapperDeployed_Inflationary" {
                atto_static_circles_to_atto_circles(*total, ts_hint)
            } else {
                *total
            };
            out.insert(*wrapper, (amount, info.token_owner));
        }
    }
    out
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
                .copied()
                .unwrap_or_else(|| Address::from_str(&edge.token_owner).unwrap_or(Address::ZERO));
            circles_types::PathfindingTransferStep {
                token_owner: format!("{token_owner:#x}"),
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
    let senders: HashSet<Address> = path.transfers.iter().map(|t| t.from).collect();
    let receivers: HashSet<Address> = path.transfers.iter().map(|t| t.to).collect();

    let source = senders
        .iter()
        .find(|a| !receivers.contains(*a))
        .copied()
        .or(override_source);
    let sink = receivers
        .iter()
        .find(|a| !senders.contains(*a))
        .copied()
        .or(override_sink);

    match (source, sink) {
        (Some(s), Some(t)) => Ok((s, t)),
        _ => Err(PathfinderError::RpcResponse(
            "Could not determine unique source/sink".into(),
        )),
    }
}
