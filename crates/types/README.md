# Circles Types

Core type definitions for the Circles protocol ecosystem.

This crate provides fundamental data structures used throughout the Circles protocol implementation, including flow matrices, transfer steps, and address handling.

## Overview

The `circles-types` crate serves as the foundation for all Circles protocol operations, providing type-safe representations of:

- Network addresses and transaction data
- Flow graph structures for pathfinding
- Transfer operations and routing information
- Matrix representations for smart contract interactions

## Features

- **Type Safety**: Leverages Rust's type system with `alloy-primitives` for Ethereum compatibility
- **Serialization**: Full `serde` support for JSON serialization/deserialization
- **Zero-Copy Operations**: Efficient memory usage with borrowed data where possible
- **Protocol Compatibility**: Types designed to match Circles smart contract interfaces

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
circles-types = "0.1.0"
```

### Basic Usage

```rust
use circles_types::{Address, TransferStep, FlowMatrix, FlowEdge, Stream};
use alloy_primitives::U256;

// Create a transfer step
let transfer = TransferStep {
    from_address: "0x123...".parse()?,
    to_address: "0x456...".parse()?,
    token_owner: "0x789...".parse()?,
    value: U256::from(1000u64),
};

// Create flow edges for routing
let edge = FlowEdge {
    stream_sink_id: 1,
    amount: U256::from(1000u64),
};

// Create streams for flow organization
let stream = Stream {
    source_coordinate: 0,
    flow_edge_ids: vec![0, 1, 2],
    data: vec![],
};
```

### Working with Flow Matrices

```rust
use circles_types::{FlowMatrix, Address};

let matrix = FlowMatrix {
    flow_vertices: vec![/* addresses */],
    flow_edges: vec![/* edges */],
    streams: vec![/* streams */],
    packed_coordinates: vec![/* coordinate data */],
    source_coordinate: 0,
};

// Flow matrices are ready for use with pathfinding algorithms
// and smart contract interactions
```

## Type Reference

### Core Types

- **`Address`**: Ethereum address type (re-exported from `alloy-primitives`)
- **`TransferStep`**: Represents a single transfer operation in a multi-hop path
- **`FlowEdge`**: Directed edge in the flow graph with amount and routing information
- **`Stream`**: Collection of flow edges representing a complete transfer route
- **`FlowMatrix`**: Complete flow representation ready for smart contract execution

### Transfer Operations

The `TransferStep` struct captures all information needed for a single transfer:

```rust
pub struct TransferStep {
    pub from_address: Address,    // Source of the transfer
    pub to_address: Address,      // Destination of the transfer
    pub token_owner: Address,     // Owner of the token being transferred
    pub value: U256,             // Amount to transfer
}
```

### Flow Graph Structures

Flow edges represent the routing information:

```rust
pub struct FlowEdge {
    pub stream_sink_id: u16,     // 0 = intermediate, 1 = terminal
    pub amount: U256,            // Amount flowing through this edge
}
```

Streams organize related flow edges:

```rust
pub struct Stream {
    pub source_coordinate: u16,   // Starting vertex index
    pub flow_edge_ids: Vec<u16>, // Edges belonging to this stream
    pub data: Vec<u8>,           // Additional protocol data
}
```

## Serialization

All types implement `Serialize` and `Deserialize` for easy JSON handling:

```rust
use circles_types::TransferStep;

let transfer = TransferStep { /* ... */ };
let json = serde_json::to_string(&transfer)?;
let parsed: TransferStep = serde_json::from_str(&json)?;
```

## Compatibility

This crate is designed to work seamlessly with:

- **`circles-pathfinder`**: Pathfinding and flow matrix calculation
- **Smart Contracts**: Types match Solidity struct layouts
- **`alloy`**: Full compatibility with the Alloy crate ecosystem
- **JSON-RPC**: Direct serialization support for protocol communication

## Contributing

Contributions are welcome! Please ensure that:

- All public types implement `Clone`, `Debug`, and serde traits where appropriate
- New types include comprehensive documentation
- Changes maintain backward compatibility where possible
- Tests cover both serialization and basic usage patterns

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
