use crate::client::RpcClient;
use crate::error::{CirclesRpcError, Result};
use crate::methods::{
    AvatarMethods, BalanceMethods, EventsMethods, GroupMethods, HealthMethods, InvitationMethods,
    NetworkMethods, PathfinderMethods, QueryMethods, SearchMethods, TablesMethods,
    TokenInfoMethods, TokenMethods, TrustMethods,
};
use crate::paged_query::{PagedFetch, PagedQuery};
use circles_types::PagedQueryParams;
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;

/// High-level facade that mirrors the TypeScript SDK entry point.
///
/// This type exposes grouped method accessors and convenience helpers for pagination
/// and streaming. It can be built from an HTTP URL or any pre-built [`RpcClient`].
pub struct CirclesRpc {
    pub client: RpcClient,
}

impl CirclesRpc {
    /// Construct from a pre-built client (useful for dependency injection in tests).
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// Build from an HTTP endpoint URL.
    pub fn from_http_url(url: reqwest::Url) -> Self {
        Self {
            client: RpcClient::http(url),
        }
    }

    /// Build from a WebSocket endpoint URL (requires the `ws` feature).
    #[cfg(feature = "ws")]
    pub async fn from_ws_url(url: reqwest::Url) -> Result<Self> {
        Ok(Self {
            client: RpcClient::ws(url).await?,
        })
    }

    /// Convenience helper to parse `&str` URLs into [`CirclesRpc`].
    pub fn try_from_http(url: &str) -> Result<Self> {
        let parsed = url
            .parse::<reqwest::Url>()
            .map_err(|e| CirclesRpcError::InvalidResponse {
                message: e.to_string(),
            })?;
        Ok(Self::from_http_url(parsed))
    }

    /// Convenience helper to parse `&str` WS URLs into [`CirclesRpc`] (requires `ws` feature).
    #[cfg(feature = "ws")]
    pub async fn try_from_ws(url: &str) -> Result<Self> {
        let parsed = url
            .parse::<reqwest::Url>()
            .map_err(|e| CirclesRpcError::InvalidResponse {
                message: e.to_string(),
            })?;
        Self::from_ws_url(parsed).await
    }

    // Method accessors
    /// RPC methods that return aggregate balances.
    pub fn balance(&self) -> BalanceMethods {
        BalanceMethods::new(self.client.clone())
    }
    /// RPC methods for token holder/balance queries.
    pub fn token(&self) -> TokenMethods {
        TokenMethods::new(self.client.clone())
    }
    /// RPC methods returning token metadata.
    pub fn token_info(&self) -> TokenInfoMethods {
        TokenInfoMethods::new(self.client.clone())
    }
    /// RPC methods for trust relations.
    pub fn trust(&self) -> TrustMethods {
        TrustMethods::new(self.client.clone())
    }
    /// RPC methods for avatar/profile lookup.
    pub fn avatar(&self) -> AvatarMethods {
        AvatarMethods::new(self.client.clone())
    }
    /// Low-level `circles_query` table accessors and pagination helpers.
    pub fn query(&self) -> QueryMethods {
        QueryMethods::new(self.client.clone())
    }
    /// RPC methods for fetching events over HTTP or websocket.
    pub fn events(&self) -> EventsMethods {
        EventsMethods::new(self.client.clone())
    }
    /// RPC methods for invitations with internal batching.
    pub fn invitation(&self) -> InvitationMethods {
        InvitationMethods::new(self.client.clone())
    }
    /// RPC methods for path-finding in the trust graph.
    pub fn pathfinder(&self) -> PathfinderMethods {
        PathfinderMethods::new(self.client.clone())
    }
    /// RPC methods for group lookups.
    pub fn group(&self) -> GroupMethods {
        GroupMethods::new(self.client.clone())
    }
    /// RPC methods for querying database tables/introspection.
    pub fn tables(&self) -> TablesMethods {
        TablesMethods::new(self.client.clone())
    }
    /// RPC methods for indexer health checks.
    pub fn health(&self) -> HealthMethods {
        HealthMethods::new(self.client.clone())
    }
    /// RPC methods for network snapshots.
    pub fn network(&self) -> NetworkMethods {
        NetworkMethods::new(self.client.clone())
    }
    /// RPC methods for profile search.
    pub fn search(&self) -> SearchMethods {
        SearchMethods::new(self.client.clone())
    }

    /// Build a `PagedQuery` helper around `circles_query`.
    pub fn paged_query<TRow>(&self, params: PagedQueryParams) -> PagedQuery<TRow>
    where
        TRow: serde::de::DeserializeOwned
            + serde::Serialize
            + Clone
            + Send
            + Sync
            + std::fmt::Debug
            + Unpin
            + 'static,
    {
        let client = self.client.clone();
        let fetch: PagedFetch<TRow> = Arc::new(move |params: PagedQueryParams| {
            let client = client.clone();
            Box::pin(async move {
                // Use the higher-level query helper to decode rows and cursors.
                QueryMethods::new(client).paged_query::<TRow>(params).await
            })
                as Pin<
                    Box<
                        dyn std::future::Future<Output = Result<circles_types::PagedResult<TRow>>>
                            + Send,
                    >,
                >
        });
        PagedQuery::new(fetch, params)
    }

    /// Convenience: directly get a stream of rows for a paged query.
    pub fn paged_stream<TRow>(&self, params: PagedQueryParams) -> impl Stream<Item = Result<TRow>>
    where
        TRow: serde::de::DeserializeOwned
            + serde::Serialize
            + Clone
            + Send
            + Sync
            + std::fmt::Debug
            + Unpin
            + 'static,
    {
        self.paged_query(params).into_stream()
    }
}

impl From<reqwest::Url> for CirclesRpc {
    fn from(url: reqwest::Url) -> Self {
        Self::from_http_url(url)
    }
}

impl TryFrom<&str> for CirclesRpc {
    type Error = CirclesRpcError;

    fn try_from(value: &str) -> Result<Self> {
        let url = value
            .parse::<reqwest::Url>()
            .map_err(|e| CirclesRpcError::InvalidResponse {
                message: e.to_string(),
            })?;
        Ok(Self::from_http_url(url))
    }
}
