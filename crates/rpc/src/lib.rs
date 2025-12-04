//! Circles RPC client: async HTTP/WS JSON-RPC wrapper mirroring the TS SDK.
//!
//! - HTTP via `alloy-provider`; WebSocket subscriptions behind the `ws` feature.
//! - Method namespaces under [`methods`] map directly to Circles RPC methods
//!   (balance, token, trust, avatar, query, events, invitation, pathfinder, group, tables, health, network, search).
//! - `paged_query`/`paged_stream` helpers for `circles_query` with cursor handling.
//! - WS parsing tolerates heartbeats (`[]`) and batched frames; unknown event types surface as `CrcUnknownEvent`.

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
