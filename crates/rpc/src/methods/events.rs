use crate::client::RpcClient;
use crate::error::Result;
use crate::events::EventStream;
use crate::events::subscription::CirclesSubscription;
use alloy_json_rpc::RpcSend;
use circles_types::{CirclesEvent, RpcSubscriptionEvent};
use futures::StreamExt;

/// Methods for fetching Circles events over HTTP or websocket.
#[derive(Clone, Debug)]
pub struct EventsMethods {
    client: RpcClient,
}

impl EventsMethods {
    /// Create a new accessor for event-related RPCs.
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// HTTP: `circles_events(address, fromBlock, toBlock?, filter?)`
    pub async fn circles_events(
        &self,
        address: Option<circles_types::Address>,
        from_block: u64,
        to_block: Option<u64>,
        filter: Option<Vec<circles_types::Filter>>,
    ) -> Result<Vec<CirclesEvent>> {
        let params = (address, from_block, to_block, filter);
        let raw: Vec<RpcSubscriptionEvent> = self.client.call("circles_events", params).await?;
        raw.into_iter()
            .map(|e| {
                crate::events::parser::parse(e).map_err(|err| {
                    crate::error::CirclesRpcError::InvalidResponse {
                        message: err.to_string(),
                    }
                })
            })
            .collect()
    }

    /// Subscribe via `eth_subscribe("circles", filter)` and yield raw `RpcSubscriptionEvent`s.
    #[cfg(feature = "ws")]
    pub async fn subscribe_circles_events<F>(
        &self,
        filter: F,
    ) -> Result<CirclesSubscription<RpcSubscriptionEvent>>
    where
        F: RpcSend + 'static,
    {
        let provider = self.client.provider().clone();
        let sub = self.client.subscribe(("circles", filter))?;
        let (stream, id) = EventStream::from_subscription(sub).await?;
        Ok(CirclesSubscription::new(stream, id, provider))
    }

    /// Subscribe and parse into `CirclesEvent` using the canonical parser.
    #[cfg(feature = "ws")]
    pub async fn subscribe_parsed_events<F>(
        &self,
        filter: F,
    ) -> Result<CirclesSubscription<CirclesEvent>>
    where
        F: RpcSend + 'static,
    {
        let provider = self.client.provider().clone();
        let sub = self.client.subscribe(("circles", filter))?;
        let (raw_stream, id) = EventStream::from_subscription(sub).await?;
        let mapped = raw_stream.into_inner().map(|item| match item {
            Ok(raw) => crate::events::parser::parse(raw).map_err(|e| {
                crate::error::CirclesRpcError::InvalidResponse {
                    message: e.to_string(),
                }
            }),
            Err(e) => Err(e),
        });
        Ok(CirclesSubscription::new(
            EventStream::new(mapped),
            id,
            provider,
        ))
    }
}
