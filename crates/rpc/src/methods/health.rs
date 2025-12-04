use crate::client::RpcClient;
use crate::error::Result;
use serde::Deserialize;

/// Health payload returned by `circles_health`.
#[derive(Clone, Debug, Deserialize)]
pub struct HealthResponse {
    /// Human-friendly status string from the indexer.
    pub status: String,
}

/// Methods for indexer health checks (`circles_health`).
#[derive(Clone, Debug)]
pub struct HealthMethods {
    client: RpcClient,
}

impl HealthMethods {
    /// Create a new accessor for health RPCs.
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// circles_health
    pub async fn health(&self) -> Result<HealthResponse> {
        self.client.call("circles_health", ()).await
    }
}
