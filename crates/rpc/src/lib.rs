//! Circles RPC client crate (work in progress).
//!
//! This crate will provide an async JSON-RPC client for the Circles protocol,
//! mirroring the TypeScript SDK surface while relying on Alloy transports and
//! shared `circles-types` definitions.

pub mod client;
pub mod error;
pub mod events;
pub mod methods;
pub mod paged_query;
pub mod rpc;
pub mod utils;

pub use client::RpcClient;
pub use error::{CirclesRpcError, Result};
pub use events::EventStream;
pub use methods::{
    AvatarMethods, BalanceMethods, EventsMethods, GroupMethods, HealthMethods, InvitationMethods,
    NetworkMethods, PathfinderMethods, QueryMethods, SearchMethods, TablesMethods,
    TokenInfoMethods, TokenMethods, TrustMethods,
};
pub use paged_query::{Page, PagedQuery};
pub use rpc::CirclesRpc;
