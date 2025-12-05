# Circles Rust Workspace

Rust implementation of the Circles SDK: JSON-RPC client, pathfinding/flow matrix tooling, transfer planning, utilities, and a higher-level `circles-sdk` orchestrator. The workspace mirrors the TypeScript SDK shape while leaning on Alloy for Ethereum primitives and transports.

## Crates at a glance
- [`circles-rpc`](crates/rpc/) — HTTP/WS JSON-RPC client with pagination helpers and event subscriptions.
- [`circles-pathfinder`](crates/pathfinder/) — pathfinding + flow matrix utilities (wrapped token handling, netted-flow checks) ready for contract calls.
- [`circles-transfers`](crates/transfers/) — builds ordered tx lists (approval → unwraps → operateFlowMatrix → inflationary re-wraps).
- [`circles-utils`](crates/utils/) — demurrage/inflation converters and day-index helpers.
- [`circles-types`](crates/types/) — shared types for RPC responses, events, pathfinding, contracts, and config.
- [`circles-sdk`](crates/sdk/) — thin orchestrator wiring RPC, profiles, pathfinding, transfers, and optional contract runners; WS helpers with retry/catch-up.
- [`crates/abis`](crates/abis/) — generated contract bindings.

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

## Docs and rustdoc
- Generate docs: `cargo doc --workspace --all-features` (add `--open` to launch the browser).
- Crate-level docs use inner `//!` comments; public APIs are documented with `///` per [rustdoc best practices](https://doc.rust-lang.org/rustdoc/what-is-rustdoc.html).
- Browse per-crate READMEs for focused examples and feature notes.

## Examples
- RPC pagination + WS:  
  `CIRCLES_RPC_URL=https://rpc.aboutcircles.com/ CIRCLES_RPC_WS_URL=wss://rpc.aboutcircles.com/ws cargo run -p circles-rpc --example paged_and_ws --features ws`
- Pathfinder contract integration:  
  `cargo run -p circles-pathfinder --example contract_integration`
- SDK examples: `cargo run -p circles-sdk --example basic_read` (see `crates/sdk/examples/` for invite generation and WS subscribe demos).

## Tests
- `cargo test -p circles-rpc`
- `cargo test -p circles-pathfinder`
- `cargo test -p circles-sdk --features ws -- --ignored` (live RPC/WS, gated by `RUN_LIVE=1` and `LIVE_AVATAR=0x...`; override endpoints with `CIRCLES_RPC_URL`, `CIRCLES_PATHFINDER_URL`, `CIRCLES_PROFILE_URL`)

## Development
- Rust 1.75+ and Cargo are required.
- Alloy versions aligned at `1.1.2` (`alloy-sol-types` 1.4.1); keep workspace dependencies in sync when bumping.
