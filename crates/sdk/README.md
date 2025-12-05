# Circles SDK (Rust)

Rust port of the Circles TypeScript SDK, orchestrating Circles RPC, profiles, pathfinding, transfers, and optional contract runners. Read flows work without a runner; write paths are gated and return `MissingRunner` until you provide one.

## Features
- Avatar helpers (human/org/group) for balances, trust, profiles, and registration flows.
- Invitation/referral helpers (generate invites, escrow/redeem checks, send/revoke/list/redeem).
- Transfer planning and replenish/max-flow helpers via `circles-transfers` and `circles-pathfinder`.
- Websocket subscriptions with retry/backoff + optional HTTP catch-up (feature `ws`).
- Shared config: `config::gnosis_mainnet()` and a `GNOSIS_MAINNET` lazy static.

## Quickstart (read-only)
```rust
use circles_sdk::{config, Sdk};
use alloy_primitives::address;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Gnosis chain (100) mainnet config
    let sdk = Sdk::new(config::gnosis_mainnet(), None)?; // runner None => read-only
    let avatar = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let info = sdk.avatar_info(avatar).await?;
    println!("avatar type: {:?}", info.avatar_type);
    Ok(())
}
```

## Examples
- `basic_read.rs`: fetch avatar info/balances/trust and run a pathfind.
- `invite_generate.rs`: build invitation secrets/signers and inspect txs.
- `ws_subscribe.rs` (feature `ws`): subscribe with retries + catch-up.

## Runners
- Implement `ContractRunner` to enable write paths (registrations, trust ops, transfers). The SDK stays read-only without one and returns `SdkError::MissingRunner`.
- Runner wiring mirrors the TS SDK; Safe support is deferred for now.

## Tests
- Unit tests: `cargo test -p circles-sdk`
- WS helpers: `cargo test -p circles-sdk --features ws`
- Optional live checks (ignored by default):  
  `RUN_LIVE=1 LIVE_AVATAR=0x... cargo test -p circles-sdk -- --ignored`  
  Override endpoints with `CIRCLES_RPC_URL`, `CIRCLES_PATHFINDER_URL`, `CIRCLES_PROFILE_URL`.

## Notes
- WS helpers tolerate heartbeats/batches and expose retry/catch-up utilities; reconnect/backoff is handled in `ws` module.
- Transfer/pathfinding helpers default to using wrapped balances; tune `AdvancedTransferOptions` if you need exclusions or simulated balances.
- Docs: `cargo doc -p circles-sdk --all-features` for rustdoc output.
