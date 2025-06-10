# Circles SDK Rust

A Rust implementation of the Circles protocol SDK, providing pathfinding, flow matrix calculation, and type-safe interactions with the Circles ecosystem.

## Overview

The Circles protocol enables a network of interconnected personal currencies, allowing users to create their own tokens and establish trust relationships for peer-to-peer transfers. This SDK provides the core building blocks for interacting with the Circles network in Rust applications.

## Features

- **Pathfinding**: Discover optimal transfer routes through the Circles trust network
- **Flow Matrix Calculation**: Generate contract-ready flow matrices for multi-hop transfers
- **Type Safety**: Strongly-typed Ethereum addresses and amounts with compile-time guarantees
- **Performance**: Zero-copy operations and efficient serialization
- **Contract Integration**: Direct compatibility with Circles smart contract ABIs
- **Well Tested**: Comprehensive test suite with integration and unit tests

## Workspace Structure

This workspace contains two main crates:

- **[`circles-types`](crates/types/)** - Core type definitions and data structures
- **[`circles-pathfinder`](crates/pathfinder/)** - Pathfinding algorithms and contract integration

## Quick Start

Add the crates to your `Cargo.toml`:

```toml
[dependencies]
circles-types = "0.1.0"
circles-pathfinder = "0.1.0"
```

### Basic Pathfinding Example

```rust
use circles_pathfinder::{prepare_flow_for_contract, FindPathParams};
use alloy_primitives::{Address, U256};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let params = FindPathParams {
        from: "0x1234567890123456789012345678901234567890".parse()?,
        to: "0x0987654321098765432109876543210987654321".parse()?,
        target_flow: U256::from(1_000_000_000_000_000_000u64), // 1 token
        use_wrapped_balances: Some(true),
        from_tokens: None,
        to_tokens: None,
        exclude_from_tokens: None,
        exclude_to_tokens: None,
    };

    let flow_matrix = prepare_flow_for_contract(
        "https://rpc.aboutcircles.com/",
        params
    ).await?;

    println!("Flow matrix ready with {} vertices", flow_matrix.flow_vertices.len());

    // Use with smart contract calls
    let (vertices, edges, streams, coordinates) = flow_matrix.into_contract_params();
    // contract.transferFlow(vertices, edges, streams, coordinates).send().await?;

    Ok(())
}
```

### Working with Types

```rust
use circles_types::{TransferStep, FlowMatrix, Address};
use alloy_primitives::U256;

let transfer = TransferStep {
    from_address: "0x123...".parse()?,
    to_address: "0x456...".parse()?,
    token_owner: "0x789...".parse()?,
    value: U256::from(1000u64),
};

// Serialize to JSON for API calls
let json = serde_json::to_string(&transfer)?;
```

## Crate Documentation

### circles-types

Core type definitions for the Circles protocol ecosystem. Provides fundamental data structures with full serde serialization support:

- `TransferStep` - Individual transfer operations
- `FlowEdge` - Directed edges in flow graphs
- `Stream` - Collections of edges representing transfer routes
- `FlowMatrix` - Complete flow representations for contracts
- `Address` - Ethereum addresses (re-exported from alloy-primitives)

**[View Documentation](crates/types/)**

### circles-pathfinder

Pathfinding algorithms and smart contract integration for the Circles network:

- Path discovery through trust networks
- Flow matrix generation for multi-hop transfers
- Contract-compatible type conversions
- Balance checking and liquidity analysis
- Coordinate packing for efficient on-chain storage

**[View Documentation](crates/pathfinder/)**

## Development

### Prerequisites

- Rust 1.75+
- Cargo

### Building

```bash
# Clone the repository
git clone https://github.com/deluXtreme/circles-rs.git
cd circles-rs

# Build all crates
cargo build

# Run tests
cargo test

# Check all crates
cargo check --workspace
```

### Running Examples

```bash
# Run pathfinder examples
cd crates/pathfinder
cargo run --example basic_pathfinding

# Run with specific RPC endpoint
RPC_URL=https://your-rpc-endpoint.com cargo run --example basic_pathfinding
```

### Testing

The workspace includes comprehensive tests:

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p circles-types
cargo test -p circles-pathfinder

# Run with output
cargo test --workspace -- --nocapture
```

## Architecture

```mermaid
graph TD
    A[circles-types] --> B[Core Types]
    A --> C[Serde Support]

    D[circles-pathfinder] --> A
    D --> E[RPC Client]
    D --> F[Flow Matrix]
    D --> G[Contract Types]

    H[Your Application] --> A
    H --> D

    D --> I[Circles RPC]
    G --> J[Smart Contracts]
```

## RPC Endpoints

The pathfinder connects to Circles RPC endpoints:

- **Mainnet**: `https://rpc.aboutcircles.com/`

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### Development Workflow

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

### Code Style

- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Follow Rust naming conventions (snake_case)
- Add documentation for public APIs

## Versioning

This project follows [Semantic Versioning](https://semver.org/). See individual crate changelogs for detailed version information:

- [circles-types changelog](crates/types/CHANGELOG.md)
- [circles-pathfinder changelog](crates/pathfinder/CHANGELOG.md)
- [Workspace changelog](CHANGELOG.md)

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

- Built on [alloy-primitives](https://github.com/alloy-rs/core) for Ethereum type compatibility
- Inspired by the [TypeScript Circles SDK](https://github.com/aboutcircles/circles-sdk)
- Part of the [Circles Protocol](https://aboutcircles.com/) ecosystem

---

**[Learn more about Circles](https://aboutcircles.com/) | [Documentation](https://docs.aboutcircles.com/)
