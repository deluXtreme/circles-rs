# Circles SDK (Rust)

Rust port of the TypeScript Circles SDK. `circles-sdk` is the high-level crate that wires together `circles-rpc`, `circles-profiles`, `circles-pathfinder`, `circles-transfers`, and the generated contract bindings.

The usage model is intentionally simple:

- Construct `Sdk` with `None` for read-only flows.
- Use `get_avatar` when you want a typed wrapper (`HumanAvatar`, `OrganisationAvatar`, `BaseGroupAvatar`).
- Provide a `ContractRunner` only when you need write paths such as registration, trust updates, or transfer submission.

## Capabilities

- Typed avatar helpers for balances, aggregated trust, profiles, direct-transfer planning/execution, pathfinding, replenish planning, group-token redeem planning/execution, registration flows, and invitation/referral discovery.
- Invitation and referral helpers for human avatars, including invitation-origin lookups, inbound/outbound invitation queries, invitation fee/module/quota helpers, proxy-inviter discovery, invite-path lookup, deterministic referral-address computation, and batch referral generation planning/execution.
- Profile metadata / short-name write helpers plus personal minting for human avatars.
- Transaction-history pagination for all typed avatars plus human group-membership/detail helpers.
- Base-group trust/property helpers (`owner`, `mint_handler`, `service`, `fee_collection`, `membership_conditions`, `trust_add_batch_with_conditions`, `set_owner`, `set_service`, `set_fee_collection`, `set_membership_condition`).
- Human and organisation group-token mint/redeem/property helpers (`plan_group_token_mint`, `mint_group_token`, `max_group_token_mintable`, `plan_group_token_redeem`, `redeem_group_token`, plus group owner/treasury/mint-handler/service/fee-collection lookups).
- Top-level SDK group convenience for `group_members`, `group_collateral`, and `group_holders`.
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
            let trust = org.aggregated_trust_relations().await?;
            println!("org trust counterparts: {}", trust.len());
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
- `invite_generate.rs`: batch referral secrets/signers plus prepared transactions.
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
- Avatar wrappers expose `total_balance`, aggregated trust helpers, `plan_direct_transfer` / `direct_transfer`, and `plan_replenish` / `replenish`; human and organisation avatars also expose `max_replenishable` plus `plan_replenish_max` / `replenish_max`.
- `HumanAvatar` now also exposes `invitation_origin`, `invited_by`, `available_invitations`, `invitations_from`, `accepted_invitees`, `pending_invitees`, `invitation_fee`, `invitation_module`, `invitation_quota`, `proxy_inviters`, `find_invite_path`, `compute_referral_address`, `plan_generate_referrals`, and `generate_referrals`.
- The SDK still uses flatter Rust methods instead of the TS object namespaces (`balances.*`, `trust.*`, `groupToken.*`), so some convenience parity remains outstanding even where the underlying capability now exists.
- The main remaining facade gaps are the optional referrals backend plus the remaining TS invitation write/planner surface (`getReferralCode` storage, direct invite construction, and full referral backend wiring), followed by wallet-backend execution parity.
- Generate local rustdoc with `cargo doc -p circles-sdk --all-features`.
