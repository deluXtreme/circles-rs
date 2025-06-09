# Circles-Pathfinder

Pathfinding and flow matrix calculation for the Circles protocol.

## Features

- **Path Discovery**: Find optimal paths between addresses in the Circles network
- **Flow Matrix Calculation**: Generate flow matrices for smart contract interactions
- **Contract Integration**: Ready-to-use types for smart contract calls
- **High Performance**: Efficient coordinate packing and vertex transformation
- **Type Safety**: Compile-time guarantees with alloy primitives

## Quick Start

### Basic Usage

```rust
use circles_pathfinder::{FindPathParams, prepare_flow_for_contract};
use alloy_primitives::{Address, U256};

let params = FindPathParams {
    from: "0x123...".parse()?,
    to: "0x456...".parse()?,
    target_flow: U256::from(1_000_000_000_000_000_000u64), // 1 ETH
    use_wrapped_balances: Some(true),
    // ... other fields
};

// One function call does everything!
let matrix = prepare_flow_for_contract("https://rpc.circles.com", params).await?;

// Ready for smart contract calls
contract.some_function(
    matrix.flow_vertices,     // Vec<Address>
    matrix.flow_edges,        // Vec<ContractFlowEdge>
    matrix.streams,           // Vec<ContractStream>
    matrix.packed_coordinates // Bytes
).send().await?;

### Advanced Usage

```rust
// For composable workflows
let transfers = circles_pathfinder::find_path(rpc_url, from, to, amount, true).await?;
let matrix = circles_pathfinder::create_flow_matrix(from, to, amount, &transfers)?;
let contract_matrix = matrix.into(); // Convert to contract types
```

## Contract ABI Compatibility

This crate provides types compatible with smart contracts that expect:

```solidity
struct FlowEdge {
    uint16 streamSinkId;
    uint192 amount;
}

struct Stream {
    uint16 sourceCoordinate;
    uint16[] flowEdgeIds;
    bytes data;
}

function redeemPayment(
    address[] memory flowVertices,
    FlowEdge[] memory flowEdges,
    Stream[] memory streams,
    bytes memory packedCoordinates
) external;
```
