use crate::avatar::common::CommonAvatar;
use crate::cid_v0_to_digest::cid_v0_to_digest;
use crate::{
    ContractRunner, Core, PreparedTransaction, Profile, SdkError, SubmittedTx, call_to_tx,
};
use abis::HubV2;
use alloy_primitives::{Address, U256, aliases::U96};
use circles_profiles::Profiles;
use circles_rpc::CirclesRpc;
#[cfg(feature = "ws")]
use circles_rpc::events::subscription::CirclesSubscription;
#[cfg(feature = "ws")]
use circles_types::CirclesEvent;
use circles_types::{
    AdvancedTransferOptions, AvatarInfo, PathfindingResult, TokenBalanceResponse, TrustRelation,
};
use std::sync::Arc;

/// Top-level avatar enum variant: organisation.
pub struct OrganisationAvatar {
    pub address: Address,
    pub info: AvatarInfo,
    pub core: Arc<Core>,
    pub runner: Option<Arc<dyn ContractRunner>>,
    pub common: CommonAvatar,
}

impl OrganisationAvatar {
    /// Get detailed token balances (v1/v2 selectable).
    pub async fn balances(
        &self,
        as_time_circles: bool,
        use_v2: bool,
    ) -> Result<Vec<TokenBalanceResponse>, SdkError> {
        self.common.balances(as_time_circles, use_v2).await
    }

    /// Get trust relations.
    pub async fn trust_relations(&self) -> Result<Vec<TrustRelation>, SdkError> {
        self.common.trust_relations().await
    }

    /// Fetch profile (cached by CID in memory).
    pub async fn profile(&self) -> Result<Option<Profile>, SdkError> {
        self.common.profile(self.info.cid_v0.as_deref()).await
    }

    /// Update profile via profiles service and store CID through NameRegistry (requires runner).
    pub async fn update_profile(&self, profile: &Profile) -> Result<Vec<SubmittedTx>, SdkError> {
        let cid = self.common.pin_profile(profile).await?;
        let digest = cid_v0_to_digest(&cid)?;
        let call = abis::NameRegistry::updateMetadataDigestCall {
            _metadataDigest: digest,
        };
        let tx = call_to_tx(self.core.config.name_registry_address, call, None);
        self.common.send(vec![tx]).await
    }

    /// Trust one or more avatars via HubV2::trust (requires runner).
    pub async fn trust_add(
        &self,
        avatars: &[Address],
        expiry: u128,
    ) -> Result<Vec<SubmittedTx>, SdkError> {
        let runner = self.runner.clone().ok_or(SdkError::MissingRunner)?;
        let txs = avatars
            .iter()
            .map(|addr| HubV2::trustCall {
                _trustReceiver: *addr,
                _expiry: U96::from(expiry),
            })
            .map(|call| call_to_tx(self.core.config.v2_hub_address, call, None))
            .collect();
        Ok(runner.send_transactions(txs).await?)
    }

    /// Remove trust (sets expiry to 0). Requires runner.
    pub async fn trust_remove(&self, avatars: &[Address]) -> Result<Vec<SubmittedTx>, SdkError> {
        self.trust_add(avatars, 0).await
    }

    #[cfg(feature = "ws")]
    pub async fn subscribe_events_ws(
        &self,
        ws_url: &str,
        filter: Option<serde_json::Value>,
    ) -> Result<CirclesSubscription<CirclesEvent>, SdkError> {
        self.common.subscribe_events_ws(ws_url, filter).await
    }

    /// Plan a transfer without submitting.
    pub async fn plan_transfer(
        &self,
        to: Address,
        amount: U256,
        options: Option<AdvancedTransferOptions>,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        self.common.plan_transfer(to, amount, options).await
    }

    /// Execute a transfer using the runner (requires runner).
    pub async fn transfer(
        &self,
        to: Address,
        amount: U256,
        options: Option<AdvancedTransferOptions>,
    ) -> Result<Vec<SubmittedTx>, SdkError> {
        self.common.transfer(to, amount, options).await
    }

    /// Find a path between this avatar and `to` with a target flow.
    pub async fn find_path(
        &self,
        to: Address,
        target_flow: U256,
        options: Option<AdvancedTransferOptions>,
    ) -> Result<PathfindingResult, SdkError> {
        self.common.find_path(to, target_flow, options).await
    }

    /// Max-flow helper: sets target_flow to U256::MAX.
    pub async fn max_flow_to(
        &self,
        to: Address,
        options: Option<AdvancedTransferOptions>,
    ) -> Result<PathfindingResult, SdkError> {
        self.common.max_flow_to(to, options).await
    }

    pub fn new(
        address: Address,
        info: AvatarInfo,
        core: Arc<Core>,
        profiles: Profiles,
        rpc: Arc<CirclesRpc>,
        runner: Option<Arc<dyn ContractRunner>>,
    ) -> Self {
        let common = CommonAvatar::new(address, core.clone(), profiles, rpc, runner.clone());
        Self {
            address,
            info,
            core,
            runner,
            common,
        }
    }
}
