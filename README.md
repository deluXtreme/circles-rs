# Circles Rust Workspace

> **Alpha release:** The API surface is still in flux; expect breaking changes between releases.

Rust implementation of the Circles SDK: JSON-RPC client, pathfinding/flow matrix tooling, transfer planning, utilities, and a higher-level `circles-sdk` orchestrator. The workspace mirrors the TypeScript SDK shape while leaning on Alloy for Ethereum primitives and transports.

The recommended entrypoint for application code is `circles-sdk`. Lower-level crates remain available when you want direct RPC access, custom pathfinding, or transfer planning without the full orchestrator.

## Crates at a glance
- [`circles-rpc`](crates/rpc/) — HTTP/WS JSON-RPC client with pagination helpers and event subscriptions.
- [`circles-pathfinder`](crates/pathfinder/) — pathfinding + flow matrix utilities (wrapped token handling, netted-flow checks) ready for contract calls.
- [`circles-transfers`](crates/transfers/) — builds ordered tx lists (approval → unwraps → operateFlowMatrix → inflationary re-wraps).
- [`circles-utils`](crates/utils/) — demurrage/inflation converters and day-index helpers.
- [`circles-types`](crates/types/) — shared types for RPC responses, events, pathfinding, contracts, and config.
- [`circles-sdk`](crates/sdk/) — thin orchestrator wiring RPC, profiles, pathfinding, transfers, and optional contract runners; WS helpers with retry/catch-up.
- [`crates/abis`](crates/abis/) — generated contract bindings.

## TypeScript parity snapshot

As of March 26, 2026, this workspace is closer to the TypeScript SDK, but it is not yet at full feature parity.

| Area | Status | Notes |
| --- | --- | --- |
| `circles-rpc` | Good coverage | Core HTTP/query/event decoding is in place, including paged transaction history plus group-membership/group-query helpers used by the higher-level crates. |
| `circles-pathfinder` | Close | Recent parity work aligned flow-matrix terminal edges, wrapped-token rewriting, token-info helpers, netted-flow helpers, and explicit RPC/client entrypoints. |
| `circles-transfers` | Close | Advanced transfer planning, aggregate transfers, and the TS-style `constructReplenish` flow are present; remaining work is mostly higher-level parity polish and broader behavioral coverage. |
| `circles-sdk` | Partial, improving | Read flows and typed avatars are usable, replenish planning/execution rides the runner abstraction, and the convenience surface now covers aggregated trust helpers, direct-transfer planning/execution, transaction-history pagination, profile metadata / short-name writes, personal minting, max-replenish helpers, base-group read/write helpers, human group-membership detail helpers, and group-token mint/property helpers; fuller group-token parity and wallet-backend parity are still incomplete. |
| `circles-profiles`, `circles-utils`, `circles-types`, `circles-abis` | Supporting / lower risk | These crates are in service for the current SDK flows and are not the main parity bottlenecks right now. |

The biggest remaining parity work is no longer the replenish planner itself. The open gaps are the rest of the higher-level SDK facade and execution-backend parity, especially fuller group-token redeem/member convenience, the richer invitation surface, and wallet-specific flows outside the generic runner abstraction.

## Usage model

- Read-only flows work with `Sdk::new(config, None)`.
- Typed avatar wrappers are discovered at runtime via `Sdk::get_avatar`.
- Write paths are delegated to a `ContractRunner` implementation instead of being hard-wired to one wallet transport.
- Lower-level crates can be used directly when you want narrower control over RPC, pathfinding, or transfer assembly.

## Quick start (read-only SDK)
```rust
use circles_sdk::{config, Sdk};
use alloy_primitives::address;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdk = Sdk::new(config::gnosis_mainnet(), None)?; // runner None => read-only
    let avatar = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let info = sdk.avatar_info(avatar).await?;
    println!("avatar type: {:?}", info.avatar_type);
    Ok(())
}
```

## Docs

- Generate docs with `cargo doc --workspace --all-features --no-deps`.
- `crates/sdk/README.md` is the best starting point for application integration.
- Per-crate READMEs cover focused examples and feature notes.

## Examples
- RPC pagination + WS:  
  `CIRCLES_RPC_URL=https://rpc.aboutcircles.com/ CIRCLES_RPC_WS_URL=wss://rpc.aboutcircles.com/ws cargo run -p circles-rpc --example paged_and_ws --features ws`
- Pathfinder contract integration:  
  `cargo run -p circles-pathfinder --example contract_integration`
- SDK examples: `cargo run -p circles-sdk --example basic_read` (see `crates/sdk/examples/` for invite generation and WS subscribe demos).

## Validation

- `cargo check`
- `cargo clippy --workspace --all-targets`
- `cargo test`
- `cargo doc --workspace --all-features --no-deps`

Live SDK checks remain opt-in:

- `RUN_LIVE=1 LIVE_AVATAR=0x... cargo test -p circles-sdk -- --ignored`
- Override endpoints with `CIRCLES_RPC_URL`, `CIRCLES_PATHFINDER_URL`, and `CIRCLES_PROFILE_URL`

## Development
- Rust 1.75+ and Cargo are required.
- Alloy versions aligned at `1.1.2` (`alloy-sol-types` 1.4.1); keep workspace dependencies in sync when bumping.
