use crate::client::RpcClient;
use crate::error::Result;
use circles_types::{Address, Balance};

/// Methods for aggregate balance queries (`circles_getTotalBalance` / `circlesV2_getTotalBalance`).
#[derive(Clone, Debug)]
pub struct BalanceMethods {
    client: RpcClient,
}

impl BalanceMethods {
    /// Create a new accessor for balance RPCs.
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// circles_getTotalBalance / circlesV2_getTotalBalance
    pub async fn get_total_balance(
        &self,
        address: Address,
        as_time_circles: bool,
        use_v2: bool,
    ) -> Result<Balance> {
        let method = if use_v2 {
            "circlesV2_getTotalBalance"
        } else {
            "circles_getTotalBalance"
        };
        self.client.call(method, (address, as_time_circles)).await
    }
}
