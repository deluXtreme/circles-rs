use crate::{Sdk, SdkError};
use alloy_primitives::Address;
use circles_types::{
    AggregatedTrustRelation, AllInvitationsResponse, AvatarInfo, TokenBalanceResponse,
};

/// Borrowed data facade mirroring the TypeScript `sdk.data.*` namespace.
pub struct Data<'a> {
    sdk: &'a Sdk,
}

impl<'a> Data<'a> {
    pub(crate) fn new(sdk: &'a Sdk) -> Self {
        Self { sdk }
    }

    /// Get avatar info for an address.
    pub async fn get_avatar(&self, avatar: Address) -> Result<AvatarInfo, SdkError> {
        self.sdk.data_avatar(avatar).await
    }

    /// Get aggregated trust relations for an address.
    ///
    /// This mirrors the current TypeScript helper name, which resolves to the
    /// aggregated trust-relations RPC method rather than raw directional rows.
    pub async fn get_trust_relations(
        &self,
        avatar: Address,
    ) -> Result<Vec<AggregatedTrustRelation>, SdkError> {
        self.sdk.data_trust_aggregated(avatar).await
    }

    /// Get token balances for an address using the default TS helper flags.
    pub async fn get_balances(
        &self,
        avatar: Address,
    ) -> Result<Vec<TokenBalanceResponse>, SdkError> {
        self.sdk.data_balances(avatar, false, true).await
    }

    /// Get all invitation sources for an address.
    pub async fn get_all_invitations(
        &self,
        avatar: Address,
        minimum_balance: Option<&str>,
    ) -> Result<AllInvitationsResponse, SdkError> {
        self.sdk.data_all_invitations(avatar, minimum_balance).await
    }
}

#[cfg(test)]
mod tests {
    use super::Data;
    use crate::config;

    #[test]
    fn data_facade_is_constructible() {
        let sdk = crate::Sdk::new(config::gnosis_mainnet(), None).expect("sdk");
        let _ = Data::new(&sdk);
    }
}
