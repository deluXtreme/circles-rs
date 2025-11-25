use crate::client::RpcClient;
use crate::error::Result;
use circles_types::{FindPathParams, PathfindingResult, SimulatedBalance};
use serde::Serialize;

/// Methods for invoking the pathfinder (max-flow) RPC.
#[derive(Clone, Debug)]
pub struct PathfinderMethods {
    client: RpcClient,
}

impl PathfinderMethods {
    /// Create a new accessor for pathfinder RPCs.
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// circlesV2_findPath — accepts full FindPathParams
    pub async fn find_path(&self, params: FindPathParams) -> Result<PathfindingResult> {
        self.client.call("circlesV2_findPath", (params,)).await
    }

    /// Token swap variant: uses the same RPC but allows specifying tokens/simulated balances.
    pub async fn find_path_with_simulation(
        &self,
        params: FindPathParams,
        simulated_balances: Option<Vec<SimulatedBalance>>,
    ) -> Result<PathfindingResult> {
        #[derive(Debug, Clone, Serialize)]
        struct Payload {
            #[serde(flatten)]
            params: FindPathParams,
            #[serde(skip_serializing_if = "Option::is_none")]
            simulated_balances: Option<Vec<SimulatedBalance>>,
        }
        let payload = Payload {
            params,
            simulated_balances,
        };
        self.client.call("circlesV2_findPath", (payload,)).await
    }
}
