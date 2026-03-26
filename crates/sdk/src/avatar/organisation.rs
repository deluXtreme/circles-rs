use crate::avatar::common::CommonAvatar;
use crate::cid_v0_to_digest::cid_v0_to_digest;
use crate::{
    ContractRunner, Core, PreparedTransaction, Profile, SdkError, SubmittedTx, call_to_tx,
};
use alloy_primitives::{Address, Bytes, U256, aliases::U96};
use circles_abis::HubV2;
use circles_profiles::Profiles;
#[cfg(feature = "ws")]
use circles_rpc::events::subscription::CirclesSubscription;
use circles_rpc::{CirclesRpc, PagedQuery};
#[cfg(feature = "ws")]
use circles_types::CirclesEvent;
use circles_types::{
    AdvancedTransferOptions, AggregatedTrustRelation, AvatarInfo, Balance, PathfindingResult,
    SortOrder, TokenBalanceResponse, TransactionHistoryRow, TrustRelation,
};
use std::sync::Arc;

/// Top-level avatar enum variant: organisation.
pub struct OrganisationAvatar {
    /// Avatar address on-chain.
    pub address: Address,
    /// RPC-derived avatar metadata.
    pub info: AvatarInfo,
    /// Shared contract bundle and configuration.
    pub core: Arc<Core>,
    /// Optional runner used for write-capable flows.
    pub runner: Option<Arc<dyn ContractRunner>>,
    /// Shared read/write helper implementation.
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

    /// Get aggregate balance (v1/v2 selectable).
    pub async fn total_balance(
        &self,
        as_time_circles: bool,
        use_v2: bool,
    ) -> Result<Balance, SdkError> {
        self.common.total_balance(as_time_circles, use_v2).await
    }

    /// Get trust relations.
    pub async fn trust_relations(&self) -> Result<Vec<TrustRelation>, SdkError> {
        self.common.trust_relations().await
    }

    /// Get aggregated trust relations.
    pub async fn aggregated_trust_relations(
        &self,
    ) -> Result<Vec<AggregatedTrustRelation>, SdkError> {
        self.common.aggregated_trust_relations().await
    }

    /// Get outgoing trust relations only.
    pub async fn trusts(&self) -> Result<Vec<AggregatedTrustRelation>, SdkError> {
        self.common.trusts().await
    }

    /// Get incoming trust relations only.
    pub async fn trusted_by(&self) -> Result<Vec<AggregatedTrustRelation>, SdkError> {
        self.common.trusted_by().await
    }

    /// Get mutual trust relations only.
    pub async fn mutual_trusts(&self) -> Result<Vec<AggregatedTrustRelation>, SdkError> {
        self.common.mutual_trusts().await
    }

    /// Check whether this avatar trusts `other_avatar`.
    pub async fn is_trusting(&self, other_avatar: Address) -> Result<bool, SdkError> {
        self.common.is_trusting(other_avatar).await
    }

    /// Check whether `other_avatar` trusts this avatar.
    pub async fn is_trusted_by(&self, other_avatar: Address) -> Result<bool, SdkError> {
        self.common.is_trusted_by(other_avatar).await
    }

    /// Fetch profile (cached by CID in memory).
    pub async fn profile(&self) -> Result<Option<Profile>, SdkError> {
        self.common.profile(self.info.cid_v0.as_deref()).await
    }

    /// Get transaction history for this avatar using cursor-based pagination.
    pub fn transaction_history(
        &self,
        limit: u32,
        sort_order: SortOrder,
    ) -> PagedQuery<TransactionHistoryRow> {
        self.common.transaction_history(limit, sort_order)
    }

    /// Update profile via profiles service and store CID through NameRegistry (requires runner).
    pub async fn update_profile(&self, profile: &Profile) -> Result<Vec<SubmittedTx>, SdkError> {
        let cid = self.common.pin_profile(profile).await?;
        self.update_profile_metadata(&cid).await
    }

    /// Update the on-chain profile CID pointer through NameRegistry (requires runner).
    pub async fn update_profile_metadata(&self, cid: &str) -> Result<Vec<SubmittedTx>, SdkError> {
        let digest = cid_v0_to_digest(cid)?;
        let call = circles_abis::NameRegistry::updateMetadataDigestCall {
            _metadataDigest: digest,
        };
        let tx = call_to_tx(self.core.config.name_registry_address, call, None);
        self.common.send(vec![tx]).await
    }

    /// Register a short name using a specific nonce (requires runner).
    pub async fn register_short_name(&self, nonce: u64) -> Result<Vec<SubmittedTx>, SdkError> {
        let call = circles_abis::NameRegistry::registerShortNameWithNonceCall {
            _nonce: U256::from(nonce),
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

    /// Plan a direct transfer without pathfinding.
    pub async fn plan_direct_transfer(
        &self,
        to: Address,
        amount: U256,
        token_address: Option<Address>,
        tx_data: Option<Bytes>,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        self.common
            .plan_direct_transfer(to, amount, token_address, tx_data)
            .await
    }

    /// Execute a direct transfer using the runner (requires runner).
    pub async fn direct_transfer(
        &self,
        to: Address,
        amount: U256,
        token_address: Option<Address>,
        tx_data: Option<Bytes>,
    ) -> Result<Vec<SubmittedTx>, SdkError> {
        self.common
            .direct_transfer(to, amount, token_address, tx_data)
            .await
    }

    /// Plan a replenish flow without submitting.
    pub async fn plan_replenish(
        &self,
        token_id: Address,
        amount: U256,
        receiver: Option<Address>,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        self.common.plan_replenish(token_id, amount, receiver).await
    }

    /// Execute a replenish flow using the runner (requires runner).
    pub async fn replenish(
        &self,
        token_id: Address,
        amount: U256,
        receiver: Option<Address>,
    ) -> Result<Vec<SubmittedTx>, SdkError> {
        self.common.replenish(token_id, amount, receiver).await
    }

    /// Compute the maximum amount that can be replenished into this organisation's own token.
    pub async fn max_replenishable(
        &self,
        options: Option<AdvancedTransferOptions>,
    ) -> Result<U256, SdkError> {
        let mut opts = options.unwrap_or(AdvancedTransferOptions {
            use_wrapped_balances: None,
            from_tokens: None,
            to_tokens: None,
            exclude_from_tokens: None,
            exclude_to_tokens: None,
            simulated_balances: None,
            simulated_trusts: None,
            max_transfers: None,
            tx_data: None,
        });
        if opts.use_wrapped_balances.is_none() {
            opts.use_wrapped_balances = Some(true);
        }
        if opts.to_tokens.is_none() {
            opts.to_tokens = Some(vec![self.address]);
        }
        Ok(self
            .common
            .find_path(self.address, U256::MAX, Some(opts))
            .await?
            .max_flow)
    }

    /// Plan a replenish flow for the maximum currently replenishable amount.
    pub async fn plan_replenish_max(
        &self,
        options: Option<AdvancedTransferOptions>,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        let max_amount = self.max_replenishable(options).await?;
        if max_amount.is_zero() {
            return Err(SdkError::OperationFailed(
                "no tokens available to replenish".to_string(),
            ));
        }
        self.plan_replenish(self.address, max_amount, None).await
    }

    /// Execute a replenish flow for the maximum currently replenishable amount.
    pub async fn replenish_max(
        &self,
        options: Option<AdvancedTransferOptions>,
    ) -> Result<Vec<SubmittedTx>, SdkError> {
        let txs = self.plan_replenish_max(options).await?;
        self.common.send(txs).await
    }

    /// Plan a group-token mint by routing collateral to the group's mint handler.
    pub async fn plan_group_token_mint(
        &self,
        group: Address,
        amount: U256,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        let mint_handler = self
            .core
            .base_group(group)
            .BASE_MINT_HANDLER()
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))?;
        self.plan_transfer(
            mint_handler,
            amount,
            Some(AdvancedTransferOptions {
                use_wrapped_balances: Some(true),
                from_tokens: None,
                to_tokens: None,
                exclude_from_tokens: None,
                exclude_to_tokens: None,
                simulated_balances: None,
                simulated_trusts: None,
                max_transfers: None,
                tx_data: None,
            }),
        )
        .await
    }

    /// Execute a group-token mint by routing collateral to the group's mint handler.
    pub async fn mint_group_token(
        &self,
        group: Address,
        amount: U256,
    ) -> Result<Vec<SubmittedTx>, SdkError> {
        let txs = self.plan_group_token_mint(group, amount).await?;
        self.common.send(txs).await
    }

    /// Plan a group-token redeem flow back into trusted treasury collateral.
    pub async fn plan_group_token_redeem(
        &self,
        group: Address,
        amount: U256,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        self.common.plan_group_token_redeem(group, amount).await
    }

    /// Execute a group-token redeem flow back into trusted treasury collateral.
    pub async fn redeem_group_token(
        &self,
        group: Address,
        amount: U256,
    ) -> Result<Vec<SubmittedTx>, SdkError> {
        self.common.group_token_redeem(group, amount).await
    }

    /// Compute the maximum amount mintable for a group from this avatar.
    pub async fn max_group_token_mintable(&self, group: Address) -> Result<U256, SdkError> {
        let mint_handler = self
            .core
            .base_group(group)
            .BASE_MINT_HANDLER()
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))?;
        Ok(self
            .max_flow_to(
                mint_handler,
                Some(AdvancedTransferOptions {
                    use_wrapped_balances: Some(true),
                    from_tokens: None,
                    to_tokens: None,
                    exclude_from_tokens: None,
                    exclude_to_tokens: None,
                    simulated_balances: None,
                    simulated_trusts: None,
                    max_transfers: None,
                    tx_data: None,
                }),
            )
            .await?
            .max_flow)
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

    /// Get the owner address for a group.
    pub async fn group_owner(&self, group: Address) -> Result<Address, SdkError> {
        self.core
            .base_group(group)
            .owner()
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
    }

    /// Get the mint handler address for a group.
    pub async fn group_mint_handler(&self, group: Address) -> Result<Address, SdkError> {
        self.core
            .base_group(group)
            .BASE_MINT_HANDLER()
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
    }

    /// Get the treasury address for a group.
    pub async fn group_treasury(&self, group: Address) -> Result<Address, SdkError> {
        self.core
            .base_group(group)
            .BASE_TREASURY()
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
    }

    /// Get the service address for a group.
    pub async fn group_service(&self, group: Address) -> Result<Address, SdkError> {
        self.core
            .base_group(group)
            .service()
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
    }

    /// Get the fee collection address for a group.
    pub async fn group_fee_collection(&self, group: Address) -> Result<Address, SdkError> {
        self.core
            .base_group(group)
            .feeCollection()
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
    }

    /// Get all membership conditions for a group.
    pub async fn group_membership_conditions(
        &self,
        group: Address,
    ) -> Result<Vec<Address>, SdkError> {
        self.core
            .base_group(group)
            .getMembershipConditions()
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
    }

    /// Build a typed organisation avatar wrapper from already-fetched components.
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Bytes, TxHash};
    use alloy_sol_types::SolCall;
    use async_trait::async_trait;
    use circles_profiles::Profiles;
    use circles_types::{AvatarType, CirclesConfig};
    use std::sync::Mutex;

    const TEST_CID: &str = "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG";

    #[derive(Default)]
    struct RecordingRunner {
        sender: Address,
        sent: Mutex<Vec<Vec<PreparedTransaction>>>,
    }

    #[async_trait]
    impl ContractRunner for RecordingRunner {
        fn sender_address(&self) -> Address {
            self.sender
        }

        async fn send_transactions(
            &self,
            txs: Vec<PreparedTransaction>,
        ) -> Result<Vec<crate::SubmittedTx>, crate::RunnerError> {
            self.sent.lock().expect("lock").push(txs.clone());
            Ok(txs
                .into_iter()
                .map(|_| crate::SubmittedTx {
                    tx_hash: Bytes::from(TxHash::ZERO.as_slice().to_vec()),
                })
                .collect())
        }
    }

    fn dummy_config() -> CirclesConfig {
        CirclesConfig {
            circles_rpc_url: "https://rpc.example.com".into(),
            pathfinder_url: "https://pathfinder.example.com".into(),
            profile_service_url: "https://profiles.example.com".into(),
            v1_hub_address: Address::repeat_byte(0x01),
            v2_hub_address: Address::repeat_byte(0x02),
            name_registry_address: Address::repeat_byte(0x03),
            base_group_mint_policy: Address::repeat_byte(0x04),
            standard_treasury: Address::repeat_byte(0x05),
            core_members_group_deployer: Address::repeat_byte(0x06),
            base_group_factory_address: Address::repeat_byte(0x07),
            lift_erc20_address: Address::repeat_byte(0x08),
            invitation_escrow_address: Address::repeat_byte(0x09),
            invitation_farm_address: Address::repeat_byte(0x0a),
            referrals_module_address: Address::repeat_byte(0x0b),
        }
    }

    fn dummy_avatar(address: Address) -> AvatarInfo {
        AvatarInfo {
            block_number: 0,
            timestamp: None,
            transaction_index: 0,
            log_index: 0,
            transaction_hash: TxHash::ZERO,
            version: 2,
            avatar_type: AvatarType::CrcV2RegisterOrganization,
            avatar: address,
            token_id: None,
            has_v1: false,
            v1_token: None,
            cid_v0_digest: None,
            cid_v0: None,
            v1_stopped: None,
            is_human: false,
            name: None,
            symbol: None,
        }
    }

    fn test_avatar() -> (OrganisationAvatar, Arc<RecordingRunner>, CirclesConfig) {
        let config = dummy_config();
        let runner = Arc::new(RecordingRunner {
            sender: Address::repeat_byte(0xbb),
            sent: Mutex::new(Vec::new()),
        });
        let avatar = OrganisationAvatar::new(
            Address::repeat_byte(0xbb),
            dummy_avatar(Address::repeat_byte(0xbb)),
            Arc::new(Core::new(config.clone())),
            Profiles::new(config.profile_service_url.clone()).expect("profiles"),
            Arc::new(CirclesRpc::try_from_http(&config.circles_rpc_url).expect("rpc")),
            Some(runner.clone()),
        );
        (avatar, runner, config)
    }

    #[tokio::test]
    async fn profile_write_helpers_encode_expected_calls() {
        let (avatar, runner, config) = test_avatar();

        avatar
            .update_profile_metadata(TEST_CID)
            .await
            .expect("update metadata");
        avatar
            .register_short_name(9)
            .await
            .expect("register short name");

        let sent = runner.sent.lock().expect("lock");
        assert_eq!(sent.len(), 2);

        assert_eq!(sent[0][0].to, config.name_registry_address);
        assert_eq!(
            &sent[0][0].data[..4],
            &circles_abis::NameRegistry::updateMetadataDigestCall {
                _metadataDigest: cid_v0_to_digest(TEST_CID).expect("cid"),
            }
            .abi_encode()[..4]
        );

        assert_eq!(sent[1][0].to, config.name_registry_address);
        assert_eq!(
            &sent[1][0].data[..4],
            &circles_abis::NameRegistry::registerShortNameWithNonceCall {
                _nonce: U256::from(9u64),
            }
            .abi_encode()[..4]
        );
    }
}
