use crate::client::RpcClient;
use crate::error::Result;
use circles_types::{Address, AggregatedTrustRelation, TrustRelation, TrustRelationType};

/// Methods for trust relation queries.
///
/// Wraps `circles_getTrustRelations` and `circles_getCommonTrust`.
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

    /// circles_getAggregatedTrustRelations
    pub async fn get_aggregated_trust_relations(
        &self,
        avatar: Address,
    ) -> Result<Vec<AggregatedTrustRelation>> {
        self.client
            .call("circles_getAggregatedTrustRelations", (avatar,))
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

    /// Filter aggregated relations to only the avatars that trust `avatar`.
    pub async fn get_trusted_by(&self, avatar: Address) -> Result<Vec<AggregatedTrustRelation>> {
        let relations = self.get_aggregated_trust_relations(avatar).await?;
        Ok(relations
            .into_iter()
            .filter(|rel| matches!(rel.relation, TrustRelationType::TrustedBy))
            .collect())
    }

    /// Filter aggregated relations to only the avatars trusted by `avatar`.
    pub async fn get_trusts(&self, avatar: Address) -> Result<Vec<AggregatedTrustRelation>> {
        let relations = self.get_aggregated_trust_relations(avatar).await?;
        Ok(relations
            .into_iter()
            .filter(|rel| matches!(rel.relation, TrustRelationType::Trusts))
            .collect())
    }

    /// Filter aggregated relations to mutual trust edges only.
    pub async fn get_mutual_trusts(&self, avatar: Address) -> Result<Vec<AggregatedTrustRelation>> {
        let relations = self.get_aggregated_trust_relations(avatar).await?;
        Ok(relations
            .into_iter()
            .filter(|rel| matches!(rel.relation, TrustRelationType::MutuallyTrusts))
            .collect())
    }
}
