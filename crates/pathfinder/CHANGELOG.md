# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2025-07-17

### Added
- Enhanced `PathData` struct with improved contract conversion methods
- Better error handling and type safety in contract integration
- Comprehensive test coverage for hub contract integration
- Zero-boilerplate contract type conversions

### Changed
- **BREAKING**: Enhanced `PathData` API with more robust conversion methods
- Improved performance in coordinate packing and vertex transformation
- Better documentation and examples for contract integration workflows

### Fixed
- Resolved edge cases in flow matrix validation
- Improved error messages for debugging contract integration issues

### Dependencies
- Updated `alloy-sol-types` for improved contract type generation
- Enhanced workspace dependency management


## [0.3.0] - 2025-01-20

### Added
- New `hub.rs` module with standard Circles Hub contract types using `sol!` macro
- `PathData` struct for simplified pathfinding result handling
- Automatic conversion methods: `to_flow_edges()`, `to_streams()`, `to_contract_params()`
- Built-in Circles Hub contract compatibility with exact ABI matching
- `FlowEdge` and `Stream` types generated from Solidity contract definitions

### Changed
- **BREAKING**: Removed `contract.rs` module and `ContractFlowMatrix` type
- **BREAKING**: `prepare_flow_for_contract()` now returns `PathData` instead of `ContractFlowMatrix`
- **BREAKING**: Users must use `path_data.to_contract_params()` for contract calls
- Simplified API eliminates manual snake_case to camelCase field conversions
- Made `hub` module public to allow direct access to contract types

### Removed
- **BREAKING**: `ContractFlowMatrix`, `ContractFlowEdge`, `ContractStream` types
- **BREAKING**: `flow_matrix_to_contract_types()` and `packed_coordinates_as_bytes()` functions
- Manual conversion boilerplate - replaced with automatic methods

### Dependencies
- Added `alloy-sol-types` for `sol!` macro support

### Documentation
- Updated README with new simplified API examples
- Added comprehensive documentation for hub contract integration
- Updated examples to demonstrate zero-boilerplate conversions

## [0.2.1] - 2025-06-10
### Documentation
- Fixed changelog formatting and content for v0.2.0 release

## [0.2.0] - 2025-06-10

### Changed
- **BREAKING**: Updated all amount types from `U256` to `U192` for better contract ABI alignment
- Switched to workspace path dependencies for `circles-types` instead of crates.io dependency
- Improved type safety and contract compatibility across all APIs

### Fixed
- Silenced dead code warnings for unused JSON-RPC protocol fields (`jsonrpc`, `id`)

### Documentation
- Removed emoji usage from markdown files for cleaner formatting
- Updated GitHub repository links to reflect correct ownership
- Removed references to non-existent testnet RPC endpoint

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

[Unreleased]: https://github.com/deluXtreme/circles-rs/compare/pathfinder-v0.1.0...HEAD
[0.1.0]: https://github.com/deluXtreme/circles-rs/releases/tag/pathfinder-v0.1.0
