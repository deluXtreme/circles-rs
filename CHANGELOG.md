# Circles SDK Rust - Workspace Changelog

This file tracks major releases and coordination between crates in the workspace.
For detailed changes, see individual crate changelogs:

- [circles-types](crates/types/CHANGELOG.md) - Core type definitions
- [circles-pathfinder](crates/pathfinder/CHANGELOG.md) - Pathfinding and flow matrix calculation

## [0.3.0] - 2025-07-17

### Coordinated Changes
- **circles-pathfinder v0.4.0**: Enhanced Circles Hub contract integration
  - Improved `PathData` API with zero-boilerplate contract conversions
  - Better type safety and error handling for contract interactions
  - Enhanced test coverage and documentation

### Workspace Improvements
- Updated contract integration patterns across workspace
- Improved developer experience with streamlined APIs
- Enhanced type safety for Circles Hub interactions

### Documentation
- Updated integration examples for improved contract workflow
- Enhanced API documentation with real-world usage patterns
- Improved error handling guidance

### Developer Experience
- Simplified contract integration with automatic type conversions
- Better debugging support with enhanced error messages
- Streamlined development workflow for contract interactions


## [0.2.1] - 2025-06-10
### Documentation
- Fixed changelog formatting and content for v0.2.0 release

## [0.2.0] - 2025-06-10

### Coordinated Changes
- **BREAKING**: Migrated all amount types from `U256` to `U192` across both crates for contract ABI alignment
  - `circles-types`: Updated `FlowEdge.amount` and `TransferStep.value` fields
  - `circles-pathfinder`: Updated all APIs to use U192 for amounts and target flows
- Implemented workspace path dependencies for better development experience
- Enhanced type safety and contract compatibility across the entire workspace

### Workspace Improvements
- Added `circles-types` as workspace dependency in root `Cargo.toml`
- Updated `circles-pathfinder` to use workspace path dependency instead of crates.io
- Established proper workspace structure for coordinated releases

### Documentation
- Removed emoji usage from all markdown files for cleaner presentation
- Updated GitHub repository links to `deluXtreme/circles-rs`
- Removed references to non-existent testnet RPC endpoints
- Improved consistency across workspace documentation

### Developer Experience
- Fixed dead code warnings in pathfinder RPC module
- Better error handling and type safety
- Streamlined development workflow with workspace dependencies

## [0.1.0] - 2025-01-XX - Initial Release

### Added
- `circles-types` v0.1.0 - Fundamental data structures for Circles protocol
- `circles-pathfinder` v0.1.0 - Pathfinding algorithms and contract integration
- Workspace configuration with shared dependencies
- Comprehensive documentation and examples

### Project Structure
- Established workspace pattern for managing interdependent crates
- Set up consistent versioning and publishing strategy
- Created modular architecture supporting both standalone and integrated usage

## Future Releases

### Planned Coordinated Release - v0.2.x
- `circles-types` v0.2.0 - Breaking change: U256 → U192 for FlowEdge amounts
- `circles-pathfinder` v0.1.1 - Compatible update to support circles-types v0.2.0

---

## Release Coordination Notes

When releasing breaking changes in `circles-types`:
1. Update and publish `circles-types` first
2. Update dependent crates to use new version
3. Test integration across workspace
4. Publish dependent crates with appropriate version bumps

[Unreleased]: https://github.com/deluXtreme/circles-rs/compare/workspace-v0.1.0...HEAD
[0.1.0]: https://github.com/deluXtreme/circles-rs/releases/tag/workspace-v0.1.0
