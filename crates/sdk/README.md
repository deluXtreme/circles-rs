# Circles SDK (Rust)

Rust port of the TypeScript Circles SDK. `circles-sdk` is the high-level crate that wires together `circles-rpc`, `circles-profiles`, `circles-pathfinder`, `circles-transfers`, and the generated contract bindings.

The usage model is intentionally simple:

- Construct `Sdk` with `None` for read-only flows.
- Use `get_avatar` when you want a typed wrapper (`HumanAvatar`, `OrganisationAvatar`, `BaseGroupAvatar`).
- Provide a `ContractRunner` only when you need write paths such as registration, trust updates, or transfer submission.

## Capabilities

- Typed avatar helpers for balances, trust, profiles, pathfinding, transfer planning, and registration flows.
- Invitation and referral helpers for human avatars.
- Transfer planning and replenish/max-flow helpers via `circles-transfers` and `circles-pathfinder`.
- Optional WebSocket subscriptions with retry/backoff and HTTP catch-up through the `ws` feature.
- Shared mainnet config through `config::gnosis_mainnet()` and `GNOSIS_MAINNET`.

## Quickstart
```rust
use alloy_primitives::address;
use circles_sdk::{Avatar, Sdk, config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdk = Sdk::new(config::gnosis_mainnet(), None)?;
    let avatar = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

    match sdk.get_avatar(avatar).await? {
        Avatar::Human(human) => {
            let balances = human.balances(false, true).await?;
            println!("human balances: {}", balances.len());
        }
        Avatar::Organisation(org) => {
            let trust = org.trust_relations().await?;
            println!("org trust edges: {}", trust.len());
        }
        Avatar::Group(group) => {
            let info = group.profile().await?;
            println!("group profile loaded: {}", info.is_some());
        }
    }

    Ok(())
}
```

## Runner Model

All write-capable methods return `SdkError::MissingRunner` until a `ContractRunner` is provided. The SDK keeps read logic separate from transaction submission so Safe, EOAs, or other runner backends can be added without changing the public read API.

## Examples

- `basic_read.rs`: avatar info, balances, trust, and pathfinding.
- `invite_generate.rs`: invitation secrets/signers plus prepared transactions.
- `ws_subscribe.rs` with `--features ws`: live events with retries and optional catch-up.

## Runners

- Implement `ContractRunner` to enable write paths.
- `PreparedTransaction` is the SDK’s handoff format: target address, calldata, and optional value.
- Safe-specific support is still deferred; the current API is intentionally generic.

## Tests

- Unit tests: `cargo test -p circles-sdk`
- WS-enabled unit tests: `cargo test -p circles-sdk --features ws`
- Optional live checks: `RUN_LIVE=1 LIVE_AVATAR=0x... cargo test -p circles-sdk -- --ignored`
- Override live endpoints with `CIRCLES_RPC_URL`, `CIRCLES_PATHFINDER_URL`, and `CIRCLES_PROFILE_URL`.

## Notes

- WS helpers tolerate heartbeats and batched frames; unknown event types still surface as regular events from `circles-rpc`.
- Transfer/pathfinding helpers default to wrapped balances; tune `AdvancedTransferOptions` when you need exclusions or simulated balances/trust edges.
- Generate local rustdoc with `cargo doc -p circles-sdk --all-features`.
