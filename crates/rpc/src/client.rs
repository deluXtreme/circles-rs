use crate::error::{CirclesRpcError, Result};
use alloy_json_rpc::{RpcRecv, RpcSend};
#[cfg(feature = "ws")]
use alloy_provider::GetSubscription;
use alloy_provider::{Identity, Provider, ProviderBuilder, RootProvider};
#[cfg(feature = "ws")]
use alloy_transport_ws::WsConnect;
use serde::de::DeserializeOwned;
use std::borrow::Cow;

/// Thin wrapper around an Alloy provider. This will be expanded with WebSocket support
/// and reconnection logic as we port the TypeScript client behavior.
#[derive(Clone, Debug)]
pub struct RpcClient {
    provider: RootProvider,
}

impl RpcClient {
    /// Create a client from an existing provider.
    pub fn new(provider: RootProvider) -> Self {
        Self { provider }
    }

    /// Build a client from an HTTP URL using the vanilla provider (no fillers).
    pub fn http(url: reqwest::Url) -> Self {
        let provider: RootProvider =
            ProviderBuilder::<Identity, Identity>::default().connect_http(url);
        Self { provider }
    }

    /// Build a client from a WebSocket URL (requires the `ws` feature).
    #[cfg(feature = "ws")]
    pub async fn ws(url: reqwest::Url) -> Result<Self> {
        let provider: RootProvider = ProviderBuilder::<Identity, Identity>::default()
            .connect_ws(WsConnect::new(url.to_string()))
            .await?;
        Ok(Self { provider })
    }

    /// Perform a JSON-RPC call using typed params and response.
    pub async fn call<Req, Resp>(&self, method: &str, params: Req) -> Result<Resp>
    where
        Req: RpcSend,
        Resp: RpcRecv + DeserializeOwned,
    {
        let method: Cow<'static, str> = Cow::Owned(method.to_string());
        self.provider
            .raw_request(method, params)
            .await
            .map_err(CirclesRpcError::from)
    }

    /// Access the inner provider. This is useful for lower-level calls or subscriptions.
    pub fn provider(&self) -> &RootProvider {
        &self.provider
    }
}

#[cfg(feature = "ws")]
impl RpcClient {
    /// Subscribe via `eth_subscribe` with arbitrary params. Returns an Alloy subscription handle.
    pub fn subscribe<P, R>(&self, params: P) -> Result<GetSubscription<P, R>>
    where
        P: RpcSend,
        R: RpcRecv,
    {
        Ok(self.provider.subscribe(params))
    }
}
