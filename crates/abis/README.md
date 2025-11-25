# Circles ABIs

Contract ABI definitions for all Circles protocol smart contracts.

This crate provides type-safe Rust bindings for all Circles smart contracts using the `alloy-sol-types` framework. Each contract module contains the complete ABI definition and generated Rust types for seamless contract interaction.

## Overview

The `abis` crate serves as the contract interface layer for the Circles protocol, providing:

- Type-safe contract bindings generated from JSON ABIs
- Complete coverage of all Circles protocol contracts
- Direct compatibility with `alloy` ecosystem for contract calls
- Compile-time verification of contract interfaces

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
abis = "0.1.0"
```

## Available Contracts

This crate includes ABI definitions for:

- **`BaseGroup`** - Base group contract functionality
- **`BaseGroupFactory`** - Factory for creating base groups
- **`DemurrageCircles`** - Demurrage-based personal currency tokens
- **`HubV2`** - Main Circles V2 protocol hub
- **`InflationaryCircles`** - Inflationary personal currency tokens
- **`InvitationEscrow`** - Escrow system for invitations
- **`InvitationFarm`** - Farming mechanism for invitations
- **`LiftERC20`** - ERC20 wrapper functionality
- **`NameRegistry`** - Name registration system
- **`ReferralsModule`** - Referral system module

## Usage

```rust
use abis::{HubV2, BaseGroup, NameRegistry};
use alloy_primitives::Address;

// Use with alloy providers
let hub_address: Address = "0x...".parse()?;
let contract = HubV2::new(hub_address, provider);

// Call contract methods
let result = contract.isHuman(user_address).call().await?;
```

## Contract Integration

All contracts are generated using `alloy-sol-types` and provide:

- **Type-safe method calls**: Compile-time verification of parameters
- **Event parsing**: Automatic decoding of contract events
- **Error handling**: Typed contract errors with detailed information
- **ABI encoding**: Automatic encoding/decoding of contract data

### Example: Checking if an address is human

```rust
use abis::HubV2;
use alloy_primitives::Address;
use alloy_provider::Provider;

async fn check_human(
    provider: impl Provider,
    hub_address: Address,
    user: Address
) -> Result<bool, Box<dyn std::error::Error>> {
    let hub = HubV2::new(hub_address, provider);
    let is_human = hub.isHuman(user).call().await?._0;
    Ok(is_human)
}
```

### Example: Listening to events

```rust
use abis::HubV2;
use alloy_primitives::Address;

// Listen for trust events
let filter = hub.Trust_filter().watch().await?;
let mut stream = filter.into_stream();

while let Some(log) = stream.next().await {
    match log {
        Ok((event, log_meta)) => {
            println!("Trust event: {} trusts {} until {}",
                event.user, event.trusted, event.expiryTime);
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

## Module Structure

Each contract follows a consistent structure:

```
contract_name/
├── mod.rs              # Contract ABI and generated types
└── contract_name.json  # Original ABI JSON file
```

The `sol!` macro generates all necessary types and methods from the JSON ABI:

```rust
use alloy_primitives::{Address, B256, Bytes, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_sol_types::{SolCall, sol};

sol!(
    HubV2,
    "src/hub_v2/hub_v2.json"
);
```

## Contract Addresses

When using these contracts, you'll need the deployed contract addresses for your target network:

- **Gnosis Chain**: Primary deployment network

Refer to the [Circles documentation](https://docs.aboutcircles.com/) for current contract addresses.

## Error Handling

Contract calls can fail for various reasons. The generated contracts provide typed errors:

```rust
use abis::HubV2;

match hub.trust(trustee, expiry_time).call().await {
    Ok(result) => println!("Trust established successfully"),
    Err(e) => {
        if let Some(revert) = e.as_revert() {
            println!("Contract reverted: {}", revert);
        } else {
            println!("Other error: {}", e);
        }
    }
}
```

## Compatibility

This crate works seamlessly with:

- **`alloy-primitives`** - For Ethereum types (Address, U256, etc.)
- **`alloy-provider`** - For blockchain connectivity
- **`alloy-contract`** - For contract interaction patterns
- **`circles-types`** - For higher-level Circles protocol types

## Development

### Adding New Contracts

To add a new contract ABI:

1. Create a new directory: `src/new_contract/`
2. Add the ABI JSON file: `src/new_contract/new_contract.json`
3. Create `src/new_contract/mod.rs` with the `sol!` macro
4. Add the module to `src/lib.rs`

### Updating ABIs

When contract ABIs change:

1. Replace the JSON file with the updated ABI
2. Recompile to generate new types
3. Update any breaking changes in dependent code

## License

Licensed under either of

- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)


## Links

- [Workspace Documentation](../../README.md)
- [Circles Protocol](https://aboutcircles.com/)
- [Alloy Documentation](https://alloy-rs.github.io/alloy/)
- [Contract Addresses](https://docs.aboutcircles.com/)
