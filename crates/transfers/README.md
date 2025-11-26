# Circles Transfers

Builder for Circles transfer transactions (port of the TS `@aboutcircles/sdk-transfers`).

## Status
- Builds the ordered transaction list: approval → unwraps → operateFlowMatrix → re-wrap inflationary leftovers.
- Handles demurraged and inflationary wrappers. Inflationary unwrap uses static amounts; leftover re-wrap uses `staticAttoCircles` from `circles_getTokenBalances`.
- Always includes a safety `setApprovalForAll` (skip-check TODO).
- Does not execute transactions; you submit the txs.

## Usage
```rust
use circles_transfers::TransferBuilder;
use circles_types::{CirclesConfig, Address};
use alloy_primitives::U256;

# async fn demo() -> Result<(), Box<dyn std::error::Error>> {
let cfg = CirclesConfig {
    circles_rpc_url: "https://rpc.aboutcircles.com/".into(),
    pathfinder_url: "".into(),
    profile_service_url: "".into(),
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

let builder = TransferBuilder::new(cfg)?;
let from: Address = "0xde374ece6fa50e781e81aac78e811b33d16912c7".parse()?;
let to: Address = "0x123400000000000000000000000000000000abcd".parse()?;
let amount = U256::from(1_000_000_000_000_000_000u64); // 1 CRC

let txs = builder
    .construct_advanced_transfer(from, to, amount, None)
    .await?;

for tx in txs {
    println!("send to: {:?}, data: 0x{}", tx.to, hex::encode(tx.data));
}
# Ok(())
# }
```

## Notes
- Requires a Circles RPC endpoint for pathfinding, token info, and balances; WS not required here.
- Inflationary wrapper re-wrap needs `staticAttoCircles` in `circles_getTokenBalances` responses (TS SDK parity). If absent, re-wrap will not run.
- Approval is always included; optional skip check is TODO.
- Self-unwrap fast-path and full replenish flow are not implemented yet.

## Caveats / TODO
- Add approval skip when already approved.
- Add self-transfer unwrap shortcut (from == to, single token pair) like TS.
- Mocked RPC tests for full flow when fixtures are available.
