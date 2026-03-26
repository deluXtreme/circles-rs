use crate::avatar::common::CommonAvatar;
use crate::cid_v0_to_digest::cid_v0_to_digest;
use crate::{
    ContractRunner, Core, PreparedTransaction, Profile, SdkError, SubmittedTx, call_to_tx,
};
use alloy_primitives::{Address, Bytes, U256, aliases::U96};
use circles_abis::BaseGroup;
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

/// Top-level avatar enum variant: base group.
pub struct BaseGroupAvatar {
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

impl BaseGroupAvatar {
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

    /// Get the total supply of this group's token.
    pub async fn total_supply(&self) -> Result<U256, SdkError> {
        let token_id = self
            .core
            .hub_v2()
            .toTokenId(self.address)
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))?;
        self.core
            .hub_v2()
            .totalSupply(token_id)
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
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

    /// Get the group owner address.
    pub async fn owner(&self) -> Result<Address, SdkError> {
        self.core
            .base_group(self.address)
            .owner()
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
    }

    /// Get the mint handler address.
    pub async fn mint_handler(&self) -> Result<Address, SdkError> {
        self.core
            .base_group(self.address)
            .BASE_MINT_HANDLER()
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
    }

    /// Get the service address.
    pub async fn service(&self) -> Result<Address, SdkError> {
        self.core
            .base_group(self.address)
            .service()
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
    }

    /// Get the fee collection address.
    pub async fn fee_collection(&self) -> Result<Address, SdkError> {
        self.core
            .base_group(self.address)
            .feeCollection()
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
    }

    /// Get all membership conditions.
    pub async fn membership_conditions(&self) -> Result<Vec<Address>, SdkError> {
        self.core
            .base_group(self.address)
            .getMembershipConditions()
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
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

    /// Update profile metadata digest on the base group (requires runner).
    pub async fn update_profile(&self, profile: &Profile) -> Result<Vec<SubmittedTx>, SdkError> {
        let cid = self.common.pin_profile(profile).await?;
        self.update_profile_metadata(&cid).await
    }

    /// Update the on-chain profile CID pointer through the BaseGroup contract (requires runner).
    pub async fn update_profile_metadata(&self, cid: &str) -> Result<Vec<SubmittedTx>, SdkError> {
        let digest = cid_v0_to_digest(cid)?;
        let call = circles_abis::BaseGroup::updateMetadataDigestCall {
            _metadataDigest: digest,
        };
        let tx = call_to_tx(self.address, call, None);
        self.common.send(vec![tx]).await
    }

    /// Register a short name using a specific nonce (requires runner).
    pub async fn register_short_name(&self, nonce: u64) -> Result<Vec<SubmittedTx>, SdkError> {
        let call = circles_abis::BaseGroup::registerShortNameWithNonceCall {
            _nonce: U256::from(nonce),
        };
        let tx = call_to_tx(self.address, call, None);
        self.common.send(vec![tx]).await
    }

    /// Trust one or more avatars via BaseGroup::trust (requires runner).
    pub async fn trust_add(
        &self,
        avatars: &[Address],
        expiry: u128,
    ) -> Result<Vec<SubmittedTx>, SdkError> {
        let runner = self.runner.clone().ok_or(SdkError::MissingRunner)?;
        let txs = avatars
            .iter()
            .map(|addr| BaseGroup::trustCall {
                _trustReceiver: *addr,
                _expiry: U96::from(expiry),
            })
            .map(|call| call_to_tx(self.address, call, None))
            .collect();
        Ok(runner.send_transactions(txs).await?)
    }

    /// Remove trust (sets expiry to 0). Requires runner.
    pub async fn trust_remove(&self, avatars: &[Address]) -> Result<Vec<SubmittedTx>, SdkError> {
        self.trust_add(avatars, 0).await
    }

    /// Trust a batch of members with membership condition checks (requires runner).
    pub async fn trust_add_batch_with_conditions(
        &self,
        avatars: &[Address],
        expiry: u128,
    ) -> Result<Vec<SubmittedTx>, SdkError> {
        let call = BaseGroup::trustBatchWithConditionsCall {
            _members: avatars.to_vec(),
            _expiry: U96::from(expiry),
        };
        let tx = call_to_tx(self.address, call, None);
        self.common.send(vec![tx]).await
    }

    /// Set a new owner for the group (requires runner).
    pub async fn set_owner(&self, owner: Address) -> Result<Vec<SubmittedTx>, SdkError> {
        let call = BaseGroup::setOwnerCall { _owner: owner };
        let tx = call_to_tx(self.address, call, None);
        self.common.send(vec![tx]).await
    }

    /// Set a new service address for the group (requires runner).
    pub async fn set_service(&self, service: Address) -> Result<Vec<SubmittedTx>, SdkError> {
        let call = BaseGroup::setServiceCall { _service: service };
        let tx = call_to_tx(self.address, call, None);
        self.common.send(vec![tx]).await
    }

    /// Set a new fee collection address for the group (requires runner).
    pub async fn set_fee_collection(
        &self,
        fee_collection: Address,
    ) -> Result<Vec<SubmittedTx>, SdkError> {
        let call = BaseGroup::setFeeCollectionCall {
            _feeCollection: fee_collection,
        };
        let tx = call_to_tx(self.address, call, None);
        self.common.send(vec![tx]).await
    }

    /// Enable or disable a membership condition (requires runner).
    pub async fn set_membership_condition(
        &self,
        condition: Address,
        enabled: bool,
    ) -> Result<Vec<SubmittedTx>, SdkError> {
        let call = BaseGroup::setMembershipConditionCall {
            _condition: condition,
            _enabled: enabled,
        };
        let tx = call_to_tx(self.address, call, None);
        self.common.send(vec![tx]).await
    }

    #[cfg(feature = "ws")]
    pub async fn subscribe_events_ws(
        &self,
        ws_url: &str,
        filter: Option<serde_json::Value>,
    ) -> Result<CirclesSubscription<CirclesEvent>, SdkError> {
        self.common.subscribe_events_ws(ws_url, filter).await
    }

    #[cfg(feature = "ws")]
    pub async fn subscribe_events_ws_with_retries(
        &self,
        ws_url: &str,
        filter: serde_json::Value,
        max_attempts: Option<usize>,
    ) -> Result<CirclesSubscription<CirclesEvent>, SdkError> {
        self.common
            .subscribe_events_ws_with_retries(ws_url, filter, max_attempts)
            .await
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

    /// Build a typed base-group avatar wrapper from already-fetched components.
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
                    success: true,
                    index: None,
                })
                .collect())
        }
    }

    fn dummy_config() -> CirclesConfig {
        CirclesConfig {
            circles_rpc_url: "https://rpc.example.com".into(),
            pathfinder_url: "https://pathfinder.example.com".into(),
            profile_service_url: "https://profiles.example.com".into(),
            referrals_service_url: None,
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
            invitation_module_address: Address::repeat_byte(0x0c),
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
            avatar_type: AvatarType::CrcV2RegisterGroup,
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

    fn test_avatar() -> (BaseGroupAvatar, Arc<RecordingRunner>) {
        let config = dummy_config();
        let runner = Arc::new(RecordingRunner {
            sender: Address::repeat_byte(0xcc),
            sent: Mutex::new(Vec::new()),
        });
        let avatar = BaseGroupAvatar::new(
            Address::repeat_byte(0xcc),
            dummy_avatar(Address::repeat_byte(0xcc)),
            Arc::new(Core::new(config.clone())),
            Profiles::new(config.profile_service_url.clone()).expect("profiles"),
            Arc::new(CirclesRpc::try_from_http(&config.circles_rpc_url).expect("rpc")),
            Some(runner.clone()),
        );
        (avatar, runner)
    }

    #[tokio::test]
    async fn base_group_write_helpers_encode_expected_calls() {
        let (avatar, runner) = test_avatar();
        let new_owner = Address::repeat_byte(0xdd);
        let new_service = Address::repeat_byte(0xee);
        let new_fee = Address::repeat_byte(0xff);
        let condition = Address::repeat_byte(0x11);

        avatar
            .update_profile_metadata(TEST_CID)
            .await
            .expect("update metadata");
        avatar
            .register_short_name(5)
            .await
            .expect("register short name");
        avatar
            .trust_add_batch_with_conditions(&[new_owner, new_service], 42)
            .await
            .expect("trust batch");
        avatar.set_owner(new_owner).await.expect("set owner");
        avatar.set_service(new_service).await.expect("set service");
        avatar
            .set_fee_collection(new_fee)
            .await
            .expect("set fee collection");
        avatar
            .set_membership_condition(condition, true)
            .await
            .expect("set membership condition");

        let sent = runner.sent.lock().expect("lock");
        assert_eq!(sent.len(), 7);

        assert_eq!(
            &sent[0][0].data[..4],
            &BaseGroup::updateMetadataDigestCall {
                _metadataDigest: cid_v0_to_digest(TEST_CID).expect("cid"),
            }
            .abi_encode()[..4]
        );
        assert_eq!(
            &sent[1][0].data[..4],
            &BaseGroup::registerShortNameWithNonceCall {
                _nonce: U256::from(5u64),
            }
            .abi_encode()[..4]
        );
        assert_eq!(
            &sent[2][0].data[..4],
            &BaseGroup::trustBatchWithConditionsCall {
                _members: vec![new_owner, new_service],
                _expiry: U96::from(42u128),
            }
            .abi_encode()[..4]
        );
        assert_eq!(
            &sent[3][0].data[..4],
            &BaseGroup::setOwnerCall { _owner: new_owner }.abi_encode()[..4]
        );
        assert_eq!(
            &sent[4][0].data[..4],
            &BaseGroup::setServiceCall {
                _service: new_service,
            }
            .abi_encode()[..4]
        );
        assert_eq!(
            &sent[5][0].data[..4],
            &BaseGroup::setFeeCollectionCall {
                _feeCollection: new_fee,
            }
            .abi_encode()[..4]
        );
        assert_eq!(
            &sent[6][0].data[..4],
            &BaseGroup::setMembershipConditionCall {
                _condition: condition,
                _enabled: true,
            }
            .abi_encode()[..4]
        );
    }
}
