# Circles Types

Complete type definitions for the Circles protocol ecosystem in Rust.

This crate provides comprehensive data structures for all aspects of the Circles protocol, including avatar management, trust relations, token operations, pathfinding, event handling, RPC communication, and contract interactions.

## Overview

The `circles-types` crate serves as the foundation for all Circles protocol operations, providing type-safe representations of the entire protocol ecosystem. It includes over 60 types organized into logical modules covering every aspect of the Circles network.

## Features

- **Complete Protocol Coverage**: Types for avatars, trust, tokens, groups, events, and more
- **Alloy Integration**: Built on `alloy-primitives` for seamless Ethereum compatibility
- **API Compatible**: Matches TypeScript Circles SDK structure exactly
- **Type Safety**: Leverages Rust's type system while maintaining flexibility
- **Serialization Support**: Full `serde` support for JSON serialization/deserialization
- **Async Ready**: Traits for contract runners and batch operations
- **Query DSL**: Complete query builder for `circles_query` RPC method

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
circles-types = "0.3.0"
```

## Quick Start

```rust
use circles_types::{
    // Core types
    Address, U256, TxHash,
    // Avatar and profile types
    AvatarInfo, Profile, AvatarType,
    // Trust relations
    TrustRelation, TrustRelationType,
    // Pathfinding
    FindPathParams, PathfindingResult,
    // Configuration
    CirclesConfig,
};

// Create avatar information
let avatar = AvatarInfo {
    block_number: 12345,
    timestamp: Some(1234567890),
    transaction_index: 1,
    log_index: 0,
    transaction_hash: "0xabc123...".parse()?,
    version: 2,
    avatar_type: AvatarType::CrcV2RegisterHuman,
    avatar: "0x123...".parse()?,
    token_id: Some(U256::from(1)),
    has_v1: false,
    v1_token: None,
    cid_v0_digest: None,
    cid_v0: None,
    v1_stopped: None,
    is_human: true,
    name: None,
    symbol: None,
};

// Create pathfinding parameters
let params = FindPathParams {
    from: "0xabc...".parse()?,
    to: "0xdef...".parse()?,
    target_flow: U256::from(1000u64),
    use_wrapped_balances: Some(true),
    from_tokens: None,
    to_tokens: None,
    exclude_from_tokens: None,
    exclude_to_tokens: None,
    simulated_balances: None,
    max_transfers: Some(10),
};

// Serialize to JSON
let json = serde_json::to_string(&avatar)?;
```

## Type Categories

### Core Blockchain Types
- `Address` - Ethereum addresses (re-exported from alloy-primitives)
- `TxHash`, `BlockHash` - Transaction and block hashes
- `U256`, `U192` - Large unsigned integers
- `TransactionRequest` - Transaction request data

### Avatar & Profile Management
- `AvatarInfo` - Complete avatar information and metadata
- `Profile` - User profile with name, description, images
- `GroupProfile` - Group profile extending Profile with symbol
- `AvatarType` - Registration event types (Human, Group, Organization)

### Trust & Social Graph
- `TrustRelation` - Individual trust relationship
- `AggregatedTrustRelation` - Processed trust relationships
- `TrustRelationType` - Trust relationship types

### Token Operations
- `TokenBalance` - Token balance with metadata
- `TokenInfo` - Token creation and type information
- `TokenHolder` - Account token holdings
- `Balance` - Flexible balance type (raw or formatted)

### Group Management
- `GroupRow` - Group registration and metadata
- `GroupMembershipRow` - Group membership records
- `GroupQueryParams` - Parameters for group queries

### Pathfinding & Transfers
- `FindPathParams` - Parameters for path computation
- `PathfindingResult` - Computed transfer path
- `TransferStep` - Individual transfer in a path
- `FlowMatrix` - Complete flow representation for contracts
- `SimulatedBalance` - Balance simulation for pathfinding

### Event System
- `CirclesEvent` - Universal event structure
- `CirclesEventType` - All supported event types (25+ variants)
- `CirclesBaseEvent` - Common event metadata

### RPC & Communication
- `JsonRpcRequest`, `JsonRpcResponse` - Standard JSON-RPC types
- `CirclesQueryResponse` - Response format for queries
- `TokenBalanceResponse` - Token balance from RPC calls

### Query System
- `QueryParams` - Parameters for `circles_query` RPC method
- `FilterPredicate`, `Conjunction` - Query filtering DSL
- `PagedResult` - Paginated query results
- `SortOrder`, `OrderBy` - Result sorting

### Contract Execution
- `ContractRunner` - Async trait for contract interactions
- `BatchRun` - Trait for batched transaction execution
- `RunnerConfig` - Configuration for contract runners

## Usage Examples

### Working with Balances

```rust
use circles_types::{Balance, U256};

let balance = Balance::Raw(U256::from(1000000000000000000u64)); // 1 token
match balance {
    Balance::Raw(amount) => println!("Raw balance: {} wei", amount),
    Balance::TimeCircles(amount) => println!("TimeCircles: {:.6}", amount),
}
```

### Query Building

```rust
use circles_types::{QueryParams, FilterPredicate, FilterType, OrderBy, SortOrder};

let query = QueryParams::new(
    "CrcV2".to_string(),
    "Avatars".to_string(),
    vec!["avatar".to_string(), "version".to_string()],
)
.with_filter(vec![
    FilterPredicate::equals("version".to_string(), 2).into()
])
.with_order(vec![
    OrderBy::desc("block_number".to_string())
])
.with_limit(100);
```

### Event Handling

```rust
use circles_types::{CirclesEvent, CirclesEventType};

// Parse events from RPC
let event: CirclesEvent = serde_json::from_str(&json_data)?;
match event.event_type {
    CirclesEventType::CrcV2RegisterHuman => {
        println!("New human registered!");
    }
    CirclesEventType::CrcV2Trust => {
        println!("Trust relationship established");
    }
    _ => println!("Other event type"),
}
```

## Flow Matrix Operations

```rust
use circles_types::{FlowMatrix, FlowEdge, Stream, TransferStep, Address, U192};

// Create transfer step
let transfer = TransferStep {
    from_address: "0x123...".parse()?,
    to_address: "0x456...".parse()?,
    token_owner: "0x789...".parse()?,
    value: U192::from(1000u64),
};

// Create flow edge
let edge = FlowEdge {
    stream_sink_id: 1,
    amount: U192::from(1000u64),
};

// Create stream
let stream = Stream {
    source_coordinate: 0,
    flow_edge_ids: vec![0],
    data: vec![],
};

// Assemble flow matrix
let matrix = FlowMatrix {
    flow_vertices: vec!["0x123...".parse()?, "0x456...".parse()?],
    flow_edges: vec![edge],
    streams: vec![stream],
    packed_coordinates: vec![0, 1, 2, 3],
    source_coordinate: 0,
};
```

## Compatibility

This crate is designed to work seamlessly with:

- **Alloy Ecosystem**: Full compatibility with alloy-primitives and alloy-rpc-types
- **Circles RPC**: Direct serialization support for all RPC methods
- **Smart Contracts**: Types match Solidity struct layouts
- **TypeScript SDK**: API-compatible with the TypeScript Circles SDK
- **Async Runtimes**: Works with tokio, async-std, and other runtimes

## Contributing

Contributions are welcome! Please ensure that:

- All public types implement appropriate traits (`Clone`, `Debug`, `Serialize`, `Deserialize`)
- New types include comprehensive documentation with examples
- API compatibility with TypeScript SDK is maintained
- Changes include appropriate tests
- Follow Rust naming conventions (snake_case)

## License

Licensed under either of

- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Links

- [Workspace Documentation](../../README.md)
- [Circles Protocol](https://aboutcircles.com/)
- [TypeScript SDK](https://github.com/aboutcircles/circles-sdk)
- [Alloy Documentation](https://alloy-rs.github.io/alloy/)
