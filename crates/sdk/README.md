# Circles SDK (Rust)

Rust port of the Circles TypeScript SDK, orchestrating Circles RPC, profiles, pathfinding, transfers, and contract runners.

## Features

- Avatar helpers (human/org/group) for balances, trust, profiles, and registration.
- Invitation/referral flows (invite generation, escrow redeem/checks).
- Transfer planning (via `circles-transfers`) and pathfinding helpers.
- Websocket subscriptions with retry/backoff and optional HTTP catch-up.

## Quickstart

```rust
use circles_sdk::{config, Sdk};
use alloy_primitives::address;
use circles_types::CirclesConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Gnosis chain (100) mainnet config
    let config: CirclesConfig = config::gnosis_mainnet();
    let sdk = Sdk::new(config, None)?;
    let avatar = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let info = sdk.avatar_info(avatar).await?;
    println!("avatar type: {:?}", info.avatar_type);
    Ok(())
}
```

Examples live under `examples/`:
- `basic_read.rs`: fetch avatar info/balances/trust and run a pathfind.
- `invite_generate.rs`: build invitation secrets/signers and inspect txs.
- `ws_subscribe.rs` (feature `ws`): subscribe with retries + catch-up.

## Tests

Unit tests are minimal; run with:
```
cargo test -p circles-sdk
cargo test -p circles-sdk --features ws
```

## Notes

- The SDK is runner-agnostic: provide a `ContractRunner` for write paths; omit for read-only.
- Pathfinding/transfer helpers default to using wrapped balances; adjust `AdvancedTransferOptions` as needed.
- WS helpers tolerate heartbeats/batches and expose retry/catch-up utilities.
