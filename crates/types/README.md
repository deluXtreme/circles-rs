# circles-types

Shared types for the Circles protocol: RPC models, events, pathfinding structures, flow matrix types, contract bindings, and configuration. Used across all crates in this workspace.

## Highlights
- Alloy-backed primitives re-exported as `Address`, `TxHash`, `U256`, `U192`, etc.
- RPC/query models: `JsonRpcRequest/Response`, `QueryParams`, `PagedQueryParams`, `FilterPredicate`, `CirclesQueryResponse`.
- Pathfinding + flow: `FindPathParams`, `PathfindingResult`, `FlowMatrix`, `TransferStep`, `SimulatedBalance`.
- Events: `CirclesEvent`, `CirclesEventType` (25+ variants) with unknown-event fallback.
- Config: `CirclesConfig` shared across SDK/RPC/pathfinder/transfers.
- Profiles/trust/tokens/groups: `AvatarInfo`, `Profile`, `TrustRelation`, `TokenInfo`, `GroupRow`, and friends.

## Quickstart
```rust
use circles_types::{
    Address, AvatarInfo, AvatarType, CirclesConfig, FindPathParams, U256,
};

let config = CirclesConfig {
    circles_rpc_url: "https://rpc.aboutcircles.com/".into(),
    pathfinder_url: "https://pathfinder.aboutcircles.com/".into(),
    profile_service_url: "https://rpc.aboutcircles.com/profiles".into(),
    v1_hub_address: Address::ZERO,
    v2_hub_address: Address::ZERO,
    name_registry_address: Address::ZERO,
    base_group_mint_policy: Address::ZERO,
    standard_treasury: Address::ZERO,
    core_members_group_deployer: Address::ZERO,
    base_group_factory_address: Address::ZERO,
    lift_erc20_address: Address::ZERO,
    invitation_escrow_address: Address::ZERO,
    invitation_farm_address: Address::ZERO,
    referrals_module_address: Address::ZERO,
};

let params = FindPathParams {
    from: "0xabc...".parse()?,
    to: "0xdef...".parse()?,
    target_flow: U256::from(1_000_000_000_000_000_000u64), // 1 CRC
    use_wrapped_balances: Some(true),
    from_tokens: None,
    to_tokens: None,
    exclude_from_tokens: None,
    exclude_to_tokens: None,
    simulated_balances: None,
    max_transfers: None,
};

let avatar = AvatarInfo {
    block_number: 0,
    timestamp: None,
    transaction_index: 0,
    log_index: 0,
    transaction_hash: Default::default(),
    version: 2,
    avatar_type: AvatarType::CrcV2RegisterHuman,
    avatar: "0x123...".parse()?,
    token_id: None,
    has_v1: false,
    v1_token: None,
    cid_v0_digest: None,
    cid_v0: None,
    v1_stopped: None,
    is_human: true,
    name: None,
    symbol: None,
};
```

## Usage notes
- Designed for `serde` round-trips against Circles RPC responses and contract bindings generated in `crates/abis`.
- Unknown event types are preserved via `CrcUnknownEvent` to keep WS/HTTP parsing resilient.
- `FindPathParams.target_flow` is `U256` (RPC contract), but downstream helpers cap to `U192` for flow matrix safety.

## Links
- Workspace overview: [`../../README.md`](../../README.md)
- TypeScript reference: https://github.com/aboutcircles/circles-sdk
- Alloy docs: https://alloy-rs.github.io/alloy/
