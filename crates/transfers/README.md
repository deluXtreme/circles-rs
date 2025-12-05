# Circles Transfers

Builder for Circles transfer transactions (port of TS `@aboutcircles/sdk-transfers`). Produces the ordered tx list; execution is up to your runner.

## Status
- Builds approval → unwraps → `operateFlowMatrix` → inflationary re-wrap leftovers.
- Handles demurraged and inflationary wrappers. Inflationary unwrap uses static amounts; leftover re-wrap uses `staticAttoCircles` from `circles_getTokenBalances`.
- Self-transfer fast-path resolves wrapper type via LiftERC20; approval inclusion is configurable.
- Fixtures cover demurraged-only, mixed wrappers (with rewrap), and a no-leftover inflationary case (static balance forced to zero for now).

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

## Notes / TODO
- Approval skip-check is configurable but still defaults to including `setApprovalForAll`.
- Inflationary no-leftover fixture can be made more realistic once timestamp/static balance data is available.
- Requires a Circles RPC endpoint for pathfinding, token info, and balances; WS not required here.
- Does not submit transactions; pair with a `ContractRunner` in the SDK to send them.
