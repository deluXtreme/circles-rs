use crate::client::RpcClient;
use crate::error::Result;
use circles_types::{FindPathParams, PathfindingResult, SimulatedBalance, SimulatedTrust};

/// Methods for invoking the pathfinder (max-flow) RPC.
///
/// Mirrors `circlesV2_findPath` and accepts the full `FindPathParams`, including
/// simulated balances, simulated trusts, and token overrides.
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

    /// Compatibility variant that lets callers overlay simulated balance/trust inputs.
    pub async fn find_path_with_simulation(
        &self,
        mut params: FindPathParams,
        simulated_balances: Option<Vec<SimulatedBalance>>,
        simulated_trusts: Option<Vec<SimulatedTrust>>,
    ) -> Result<PathfindingResult> {
        if simulated_balances.is_some() {
            params.simulated_balances = simulated_balances;
        }
        if simulated_trusts.is_some() {
            params.simulated_trusts = simulated_trusts;
        }
        self.find_path(params).await
    }
}
