use crate::client::RpcClient;
use crate::error::Result;
use circles_types::{Address, TrustRelation, TrustRelationType};

/// Methods for trust relation queries.
#[derive(Clone, Debug)]
pub struct TrustMethods {
    client: RpcClient,
}

impl TrustMethods {
    /// Create a new accessor for trust-related RPCs.
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// circles_getTrustRelations
    pub async fn get_trust_relations(&self, address: Address) -> Result<Vec<TrustRelation>> {
        self.client
            .call("circles_getTrustRelations", (address,))
            .await
    }

    /// circles_getCommonTrust
    pub async fn get_common_trust(
        &self,
        avatar_a: Address,
        avatar_b: Address,
    ) -> Result<Vec<TrustRelationType>> {
        self.client
            .call("circles_getCommonTrust", (avatar_a, avatar_b))
            .await
    }
}
