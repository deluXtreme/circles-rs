# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-01-XX

### Added

#### Core Pathfinding
- `find_path()` function for discovering optimal paths between addresses in the Circles network
- `find_path_with_params()` function accepting structured `FindPathParams` for cleaner API
- `FindPathParams` struct for organized parameter passing with future extensibility
- Support for wrapped balances and basic path filtering
- Integration with Circles RPC endpoints

#### Flow Matrix Calculation
- `create_flow_matrix()` function for generating flow matrices from transfer paths
- Coordinate packing utilities (`pack_coordinates()`, `transform_to_flow_vertices()`)
- Deterministic vertex sorting for consistent results
- Terminal edge detection and validation
- Flow balance verification with detailed error reporting

#### Contract Integration
- `ContractFlowMatrix` struct with contract-ready types matching smart contract ABIs
- `ContractFlowEdge` and `ContractStream` types compatible with Solidity structs
- Automatic type conversions from internal types to contract types
- `prepare_flow_for_contract()` high-level convenience function
- `prepare_flow_for_contract_simple()` for backwards compatibility
- `get_available_flow()` helper for liquidity checking
- Support for tuple-based contract parameter decomposition

#### Error Handling
- `PathfinderError` enum with detailed error variants:
  - `Imbalanced` for flow validation errors
  - `Rpc` for network communication errors  
  - `JsonRpc` for RPC protocol errors
- Comprehensive error context and debugging information
- Graceful handling of partial liquidity scenarios

#### Developer Experience
- Comprehensive test suite with integration, unit, and contract interaction tests
- Example code demonstrating both simple and advanced usage patterns
- Detailed documentation with code examples
- Type-safe API using `alloy-primitives` for Ethereum types
- Zero-copy operations where possible for performance

#### Utilities
- Coordinate packing for efficient on-chain storage
- Address deduplication and sorting
- Flow vertex transformation and indexing
- Bytes conversion utilities for contract interactions

### Technical Details
- Built on `alloy-primitives` for Ethereum type compatibility
- Async/await support with `tokio` runtime
- JSON-RPC client implementation using `reqwest`
- Comprehensive error handling with `thiserror`
- Modular architecture supporting both high-level and composable APIs

[Unreleased]: https://github.com/aboutcircles/circles-sdk/compare/pathfinder-v0.1.0...HEAD
[0.1.0]: https://github.com/aboutcircles/circles-sdk/releases/tag/pathfinder-v0.1.0