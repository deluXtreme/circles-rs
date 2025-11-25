//! # Circles Types
//!
//! Complete type definitions for the Circles protocol ecosystem in Rust.
//!
//! This crate provides comprehensive data structures for all aspects of the Circles
//! protocol, including avatar management, trust relations, token operations, pathfinding,
//! event handling, RPC communication, and contract interactions. All types support
//! full `serde` serialization and are compatible with the TypeScript Circles SDK.
//!
//! ## Features
//!
//! - **Complete Protocol Coverage**: Types for avatars, trust, tokens, groups, events
//! - **Alloy Integration**: Built on `alloy-primitives` for Ethereum compatibility
//! - **API Compatible**: Matches TypeScript SDK structure exactly
//! - **Type Safety**: Leverages Rust's type system while maintaining flexibility
//! - **Async Ready**: Traits for contract runners and batch operations
//! - **Query DSL**: Complete query builder for `circles_query` RPC method
//!
//! ## Usage Examples
//!
//! ```rust,ignore
//! use circles_types::{
//!     // Core types
//!     Address, U256, TxHash,
//!     // Avatar and profile types
//!     AvatarInfo, Profile, AvatarType,
//!     // Pathfinding
//!     FindPathParams, PathfindingResult,
//!     // Trust relations
//!     TrustRelation, TrustRelationType,
//!     // Configuration
//!     CirclesConfig,
//! };
//!
//! // Create avatar information
//! let avatar = AvatarInfo {
//!     block_number: 12345,
//!     timestamp: Some(1234567890),
//!     transaction_index: 1,
//!     log_index: 0,
//!     transaction_hash: "0xabc123...".parse()?,
//!     version: 2,
//!     avatar_type: AvatarType::CrcV2RegisterHuman,
//!     avatar: "0x123...".parse()?,
//!     token_id: Some(U256::from(1)),
//!     has_v1: false,
//!     v1_token: None,
//!     cid_v0_digest: None,
//!     cid_v0: None,
//!     v1_stopped: None,
//!     is_human: true,
//!     name: None,
//!     symbol: None,
//! };
//!
//! // Create pathfinding parameters
//! let params = FindPathParams {
//!     from: "0xabc...".parse()?,
//!     to: "0xdef...".parse()?,
//!     target_flow: U256::from(1000u64),
//!     use_wrapped_balances: Some(true),
//!     from_tokens: None,
//!     to_tokens: None,
//!     exclude_from_tokens: None,
//!     exclude_to_tokens: None,
//!     simulated_balances: None,
//!     max_transfers: Some(10),
//! };
//!
//! // Serialize to JSON
//! let json = serde_json::to_string(&avatar)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Type Categories
//!
//! ### Core Blockchain Types
//! - [`Address`] - Ethereum address (re-exported from alloy-primitives)
//! - [`TxHash`], [`BlockHash`] - Transaction and block hashes
//! - [`U256`], [`U192`] - Large unsigned integers
//! - [`TransactionRequest`] - Transaction request data
//!
//! ### Avatar & Profile Management
//! - [`AvatarInfo`] - Complete avatar information and metadata
//! - [`Profile`] - User profile with name, description, images
//! - [`GroupProfile`] - Group profile extending Profile with symbol
//! - [`AvatarType`] - Registration event types (Human, Group, Organization)
//!
//! ### Trust & Social Graph
//! - [`TrustRelation`] - Individual trust relationship
//! - [`AggregatedTrustRelation`] - Processed trust relationships
//! - [`TrustRelationType`] - Trust relationship types
//!
//! ### Token Operations
//! - [`TokenBalance`] - Token balance with metadata
//! - [`TokenInfo`] - Token creation and type information
//! - [`TokenHolder`] - Account token holdings
//! - [`Balance`] - Flexible balance type (raw or formatted)
//!
//! ### Group Management
//! - [`GroupRow`] - Group registration and metadata
//! - [`GroupMembershipRow`] - Group membership records
//! - [`GroupQueryParams`] - Parameters for group queries
//!
//! ### Pathfinding & Transfers
//! - [`FindPathParams`] - Parameters for path computation
//! - [`PathfindingResult`] - Computed transfer path
//! - [`TransferStep`] - Individual transfer in a path
//! - [`FlowMatrix`] - Complete flow representation for contracts
//! - [`SimulatedBalance`] - Balance simulation for pathfinding
//!
//! ### Event System
//! - [`CirclesEvent`] - Universal event structure
//! - [`CirclesEventType`] - All supported event types (25+ variants)
//! - [`CirclesBaseEvent`] - Common event metadata
//!
//! ### RPC & Communication
//! - [`JsonRpcRequest`], [`JsonRpcResponse`] - Standard JSON-RPC types
//! - [`CirclesQueryResponse`] - Response format for queries
//! - [`TokenBalanceResponse`] - Token balance from RPC calls
//!
//! ### Query System
//! - [`QueryParams`] - Parameters for `circles_query` RPC method
//! - [`FilterPredicate`], [`Conjunction`] - Query filtering DSL
//! - [`PagedResult`] - Paginated query results
//! - [`SortOrder`], [`OrderBy`] - Result sorting
//!
//! ### Contract Execution
//! - [`ContractRunner`] - Async trait for contract interactions
//! - [`BatchRun`] - Trait for batched transaction execution
//! - [`RunnerConfig`] - Configuration for contract runners
//!
//! ### Protocol Configuration
//! - [`CirclesConfig`] - Complete protocol configuration
//! - [`EscrowedAmountAndDays`] - Contract-specific response types
//! - [`DecodedContractError`] - Contract error information
//!
//! ### Network State
//! - [`NetworkSnapshot`] - Complete network state at a block
//! - [`EventRow`] - Base structure for event pagination
//! - [`Cursor`] - Pagination cursor for efficient queries

// =============================================================================
// External re-exports
// =============================================================================

// Alloy primitive types
pub use alloy_primitives::aliases::{BlockHash, TxHash};
pub use alloy_primitives::Address;
pub use alloy_primitives::Bytes;
pub use alloy_primitives::{aliases::U192, U256};

// Alloy RPC types
pub use alloy_provider::transport::TransportResult;
pub use alloy_rpc_types::TransactionRequest;

// =============================================================================
// Internal modules with explicit re-exports
// =============================================================================

mod avatar;
pub use avatar::{AvatarInfo, AvatarType, GeoLocation, GroupProfile, Profile};

mod config;
pub use config::CirclesConfig;

mod contracts;
pub use contracts::EscrowedAmountAndDays;

mod errors;
pub use errors::DecodedContractError;

mod events;
pub use events::{CirclesBaseEvent, CirclesEvent, CirclesEventType, RpcSubscriptionEvent};

mod group;
pub use group::{GroupMembershipRow, GroupQueryParams, GroupRow};

mod network;
pub use network::{EventType, NetworkSnapshot};

mod trust;
pub use trust::{AggregatedTrustRelation, TrustRelation, TrustRelationType};

mod wrapper;
pub use wrapper::{CirclesType, WrappedTokenInfo, WrappedTokensRecord};

mod token;
pub use token::{TokenBalance, TokenHolder, TokenInfo};

mod client;
pub use client::{AvatarRow, CirclesQuery, GroupType, TokenBalanceRow, TrustRelationRow};

mod runner;
pub use runner::{BatchRun, ContractRunner, RunnerConfig};

mod rpc;
pub use rpc::{
    Balance, CirclesQueryResponse, JsonRpcError, JsonRpcRequest, JsonRpcResponse, QueryResponse,
    SafeQueryResponse, TokenBalanceResponse,
};

mod query;
pub use query::{
    ColumnInfo, Conjunction, ConjunctionType, Cursor, EventRow, Filter, FilterPredicate,
    FilterType, OrderBy, PagedQueryParams, PagedResult, QueryParams, SortOrder, TableInfo,
};

mod pathfinding;
pub use pathfinding::{
    AdvancedTransferOptions,
    FindPathParams,
    // Original flow types
    FlowEdge,
    FlowEdgeStruct,
    FlowMatrix,
    PathfindingFlowMatrix,
    PathfindingResult,
    PathfindingTransferStep,
    SimulatedBalance,
    Stream,
    StreamStruct,
    TransferStep,
};
