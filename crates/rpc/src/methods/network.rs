use crate::client::RpcClient;
use crate::error::Result;
use circles_types::NetworkSnapshot;

/// Methods for fetching network snapshots (`circles_getNetworkSnapshot`).
#[derive(Clone, Debug)]
pub struct NetworkMethods {
    client: RpcClient,
}

impl NetworkMethods {
    /// Create a new accessor for network snapshot RPCs.
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// circles_getNetworkSnapshot
    pub async fn snapshot(&self) -> Result<NetworkSnapshot> {
        self.client.call("circles_getNetworkSnapshot", ()).await
    }
}
