# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-06-10

### Changed
- **BREAKING**: Changed `FlowEdge.amount` and `TransferStep.value` fields from `U256` to `U192` to match contract ABI specification
- Improved type safety and contract compatibility across all amount fields

## [0.1.0] - 2025-06-09

### Added
- Core type definitions for Circles protocol ecosystem
- `TransferStep` struct for individual transfer operations with Ethereum addresses and U192 values
- `FlowEdge` struct for flow graph edges with U192 amounts and stream sink IDs
- `Stream` struct for transfer route collections with coordinates and edge references
- `FlowMatrix` struct for complete flow representations ready for contract execution
- Full serde serialization/deserialization support for all types
- Re-export of `Address` type from alloy-primitives for Ethereum address handling
- Comprehensive documentation with usage examples
- Type safety with strongly-typed addresses instead of strings

### Technical Details
- Built on `alloy-primitives` for Ethereum type compatibility
- Uses U192 for amounts to match smart contract ABI constraints
- Supports efficient serialization for JSON-RPC communication
- Designed for zero-copy operations where possible

[Unreleased]: https://github.com/deluXtreme/circles-rs/compare/types-v0.1.0...HEAD
[0.1.0]: https://github.com/deluXtreme/circles-rs/releases/tag/types-v0.1.0
