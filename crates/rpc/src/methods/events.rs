use crate::client::RpcClient;
use crate::error::Result;
use crate::events::EventStream;
use crate::events::subscription::CirclesSubscription;
use alloy_json_rpc::RpcSend;
use circles_types::{CirclesEvent, RpcSubscriptionEvent};
use futures::StreamExt;

/// Methods for fetching Circles events over HTTP or websocket.
///
/// WS helpers subscribe to `eth_subscribe("circles", filter)`, drop heartbeat
/// frames (`[]`), flatten batches, and parse into `CirclesEvent` when desired.
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
        // Subscribe as serde_json::Value to tolerate keep-alive frames like `[]`
        // that some public endpoints emit, then map into RpcSubscriptionEvent.
        let provider = self.client.provider().clone();
        let sub = self
            .client
            .subscribe::<_, serde_json::Value>(("circles", filter))?;
        let (raw_stream, id) = EventStream::from_subscription(sub).await?;
        let mapped = raw_stream.into_inner().flat_map(|item| match item {
            Ok(val) => {
                // Normalize frames: empty arrays are heartbeats, arrays batch events.
                if let Some(arr) = val.as_array() {
                    if arr.is_empty() {
                        return futures::stream::empty().boxed();
                    }
                    let iter = arr.clone().into_iter().map(|v| {
                        serde_json::from_value::<RpcSubscriptionEvent>(v).map_err(|err| {
                            crate::error::CirclesRpcError::InvalidResponse {
                                message: err.to_string(),
                            }
                        })
                    });
                    return futures::stream::iter(iter).boxed();
                }
                futures::stream::once(async {
                    serde_json::from_value::<RpcSubscriptionEvent>(val).map_err(|err| {
                        crate::error::CirclesRpcError::InvalidResponse {
                            message: err.to_string(),
                        }
                    })
                })
                .boxed()
            }
            Err(e) => futures::stream::once(async { Err(e) }).boxed(),
        });
        Ok(CirclesSubscription::new(
            EventStream::new(mapped),
            id,
            provider,
        ))
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
        let sub = self
            .client
            .subscribe::<_, serde_json::Value>(("circles", filter))?;
        let (raw_stream, id) = EventStream::from_subscription(sub).await?;
        let mapped = raw_stream.into_inner().flat_map(|item| match item {
            Ok(val) => {
                if let Some(arr) = val.as_array() {
                    if arr.is_empty() {
                        return futures::stream::empty().boxed();
                    }
                    let iter = arr.clone().into_iter().map(|v| {
                        serde_json::from_value::<RpcSubscriptionEvent>(v)
                            .map_err(|e| crate::error::CirclesRpcError::InvalidResponse {
                                message: e.to_string(),
                            })
                            .and_then(|raw| {
                                crate::events::parser::parse(raw).map_err(|e| {
                                    crate::error::CirclesRpcError::InvalidResponse {
                                        message: e.to_string(),
                                    }
                                })
                            })
                    });
                    return futures::stream::iter(iter).boxed();
                }
                futures::stream::once(async {
                    serde_json::from_value::<RpcSubscriptionEvent>(val)
                        .map_err(|e| crate::error::CirclesRpcError::InvalidResponse {
                            message: e.to_string(),
                        })
                        .and_then(|raw| {
                            crate::events::parser::parse(raw).map_err(|e| {
                                crate::error::CirclesRpcError::InvalidResponse {
                                    message: e.to_string(),
                                }
                            })
                        })
                })
                .boxed()
            }
            Err(e) => futures::stream::once(async { Err(e) }).boxed(),
        });
        Ok(CirclesSubscription::new(
            EventStream::new(mapped),
            id,
            provider,
        ))
    }
}
