# circles-pathfinder

Pathfinding and flow matrix calculation for the Circles protocol. Uses `circles-rpc` to call `circlesV2_findPath` and produces contract-ready types for Hub interactions.

## Features
- Path discovery through the trust network with optional wrapped-balance usage and simulated balances.
- Flow matrix creation and packing for on-chain `operateFlowMatrix` / `redeemPayment`.
- Wrapped token handling helpers: normalize wrappers, unwrap inflationary balances, and rewrite paths to underlying avatars.
- Netted-flow checks: shrink path values, compute/validate netted flow, and cap `U256` inputs to `U192` for contract compatibility.
- Contract-ready conversions via `sol!` types (FlowEdge/Stream) and coordinate packing helpers.

## Quickstart
```rust
use circles_pathfinder::{prepare_flow_for_contract, FindPathParams};
use alloy_primitives::{Address, U256};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let params = FindPathParams {
        from: "0x1234567890123456789012345678901234567890".parse()?,
        to: "0x0987654321098765432109876543210987654321".parse()?,
        target_flow: U256::from(1_000_000_000_000_000_000u64), // 1 CRC (U256 input)
        use_wrapped_balances: Some(true),
        from_tokens: None,
        to_tokens: None,
        exclude_from_tokens: None,
        exclude_to_tokens: None,
        simulated_balances: None,
        max_transfers: None,
    };

    // One call does everything: RPC pathfind + flow matrix ready for contracts
    let path_data = prepare_flow_for_contract("https://rpc.aboutcircles.com/", params).await?;
    let (vertices, edges, streams, coords) = path_data.into_contract_params();
    println!("matrix: {} vertices, {} edges", vertices.len(), edges.len());
}
```

## Wrapped tokens & netted flow
- Replace wrapped token owners with the underlying avatar for contract-facing flows:
```rust
use circles_pathfinder::{
    token_info_map_from_path, wrapped_totals_from_path, expected_unwrapped_totals,
    replace_wrapped_tokens, compute_netted_flow, assert_no_netted_flow_mismatch,
};

let info_map = token_info_map_from_path(current_avatar, &rpc, &path).await?;
let wrapped = wrapped_totals_from_path(&path, &info_map);
let unwrapped = expected_unwrapped_totals(&wrapped, &info_map); // uses converter + timestamp hint
let rewritten = replace_wrapped_tokens(&path, &unwrapped);
compute_netted_flow(&rewritten);
assert_no_netted_flow_mismatch(&rewritten, None, None)?;
```
- Helpers normalize wrapper variants, unwrap inflationary balances with `circles-utils`, and ensure flow matrix amounts fit in `U192` (`u256_to_u192`).

## Examples
- `contract_integration`: end-to-end pathfinding and flow matrix creation for contract calls.
- `find_path`: basic pathfind against a Circles RPC endpoint.
- `path_and_events`: pathfind plus optional WS event subscription (`CIRCLES_RPC_URL`, `CIRCLES_RPC_WS_URL`). WS parsing tolerates heartbeats/batches; unknown events become `CrcUnknownEvent`.

## Notes / status
- Targets Circles v2 RPCs; wrapper conversion logic matches the TypeScript `@pathfinder` reference.
- Use `circles-utils` demurrage/inflation converters when inspecting wrapped balances directly.
- Subscription reconnect/backoff is not handled here; use the SDK crate if you need retries or catch-up helpers.
