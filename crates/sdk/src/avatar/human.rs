use crate::avatar::common::CommonAvatar;
use crate::cid_v0_to_digest::cid_v0_to_digest;
use crate::runner::{PreparedTransaction as RunnerTx, SubmittedTx as RunnerSubmitted};
use crate::services::referrals::{
    ReferralPreviewList, ReferralPublicListOptions, Referrals, generate_private_key,
    private_key_to_address,
};
use crate::{
    ContractRunner, Core, PreparedTransaction, Profile, SdkError, SubmittedTx, call_to_tx,
};
use alloy_primitives::{Address, Bytes, U256, address, aliases::U96};
use alloy_sol_types::{SolCall, SolValue, sol};
use circles_abis::{HubV2, InvitationFarm, ReferralsModule};
use circles_profiles::Profiles;
#[cfg(feature = "ws")]
use circles_rpc::events::subscription::CirclesSubscription;
use circles_rpc::{CirclesRpc, PagedQuery};
use circles_transfers::TransferBuilder;
#[cfg(feature = "ws")]
use circles_types::CirclesEvent;
use circles_types::{
    AdvancedTransferOptions, AggregatedTrustRelation, AllInvitationsResponse, AtScaleInvitation,
    AvatarInfo, Balance, EscrowInvitation, GroupMembershipRow, GroupQueryParams, GroupRow,
    InvitationOriginResponse, InvitationsFromResponse, InvitedAccountInfo, PathfindingResult,
    PathfindingTransferStep, SimulatedTrust, SortOrder, TokenBalanceResponse,
    TransactionHistoryRow, TrustInvitation, TrustRelation,
};
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use std::sync::Arc;

/// Top-level avatar enum variant: human.
pub struct HumanAvatar {
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

/// Invitation generation result (secrets + signers + prepared txs).
#[derive(Debug, Clone)]
pub struct GeneratedInvites {
    /// Generated invitation secrets in hex form.
    pub secrets: Vec<String>,
    /// Derived signer addresses associated with the generated secrets.
    pub signers: Vec<Address>,
    /// Prepared transactions required to mint/fund the invites.
    pub txs: Vec<RunnerTx>,
    /// Submitted transactions when invite generation was executed through a runner.
    pub submitted: Option<Vec<RunnerSubmitted>>,
}

/// Single referral-code planning result for the TS `getReferralCode` flow.
#[derive(Debug, Clone)]
pub struct ReferralCodePlan {
    /// Generated referral private key to share with the invitee.
    pub private_key: String,
    /// Signer address derived from the generated private key.
    pub signer: Address,
    /// Prepared transactions required to activate the referral on-chain.
    pub txs: Vec<PreparedTransaction>,
}

/// Proxy inviter candidate used by the TS invitation planner surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyInviter {
    pub address: Address,
    pub possible_invites: u64,
}

sol! {
    struct ReferralPayload {
        address referralsModule;
        bytes callData;
    }

    #[sol(rpc)]
    contract SafeMinimal {
        function isModuleEnabled(address module) external view returns (bool);
        function enableModule(address module) external;
    }

    #[sol(rpc)]
    contract InvitationModuleMinimal {
        function trustInviter(address inviter) external;
    }
}

fn invitation_fee_amount() -> U256 {
    U256::from(96u128) * U256::from(10).pow(U256::from(18))
}

fn invitation_max_flow() -> U256 {
    U256::from_str("9999999999999999999999999999999999999").expect("valid invitation max flow")
}

fn gnosis_group_address() -> Address {
    address!("c19bc204eb1c1d5b3fe500e5e5dfabab625f286c")
}

fn farm_destination_address() -> Address {
    address!("9Eb51E6A39B3F17bB1883B80748b56170039ff1d")
}

fn farm_quota_holder() -> Address {
    address!("20EcD8bDeb2F48d8a7c94E542aA4feC5790D9676")
}

fn encode_direct_invite_data(invitee: Address) -> Bytes {
    Bytes::from(invitee.abi_encode())
}

fn build_enable_module_tx(inviter: Address, invitation_module: Address) -> PreparedTransaction {
    let call = SafeMinimal::enableModuleCall {
        module: invitation_module,
    };
    call_to_tx(inviter, call, None)
}

fn build_trust_inviter_tx(invitation_module: Address, inviter: Address) -> PreparedTransaction {
    let call = InvitationModuleMinimal::trustInviterCall { inviter };
    call_to_tx(invitation_module, call, None)
}

fn build_inviter_setup_txs(
    inviter: Address,
    invitation_module: Address,
    module_enabled: bool,
    inviter_trusted: bool,
) -> Vec<PreparedTransaction> {
    let mut txs = Vec::new();

    if !module_enabled {
        txs.push(build_enable_module_tx(inviter, invitation_module));
        txs.push(build_trust_inviter_tx(invitation_module, inviter));
    } else if !inviter_trusted {
        txs.push(build_trust_inviter_tx(invitation_module, inviter));
    }

    txs
}

fn transfer_txs_to_prepared(txs: Vec<circles_transfers::TransferTx>) -> Vec<PreparedTransaction> {
    txs.into_iter()
        .map(|tx| PreparedTransaction {
            to: tx.to,
            data: tx.data,
            value: Some(tx.value),
        })
        .collect()
}

fn build_claim_invite_tx(invitation_farm: Address) -> PreparedTransaction {
    let call = InvitationFarm::claimInviteCall {};
    call_to_tx(invitation_farm, call, None)
}

fn encode_referral_invite_data(signer: Address, referrals_module: Address) -> Bytes {
    let create_account = ReferralsModule::createAccountCall { signer };
    let payload = ReferralPayload {
        referralsModule: referrals_module,
        callData: create_account.abi_encode().into(),
    };
    Bytes::from(payload.abi_encode())
}

fn build_invitation_transfer_tx(
    hub: Address,
    from: Address,
    invitation_module: Address,
    claimed_id: U256,
    tx_data: Bytes,
) -> PreparedTransaction {
    let call = HubV2::safeTransferFromCall {
        _from: from,
        _to: invitation_module,
        _id: claimed_id,
        _value: invitation_fee_amount(),
        _data: tx_data,
    };
    call_to_tx(hub, call, None)
}

fn order_proxy_inviters(mut inviters: Vec<ProxyInviter>, inviter: Address) -> Vec<ProxyInviter> {
    inviters.sort_by(|a, b| {
        let a_is_inviter = a.address == inviter;
        let b_is_inviter = b.address == inviter;
        b_is_inviter
            .cmp(&a_is_inviter)
            .then_with(|| format!("{:#x}", a.address).cmp(&format!("{:#x}", b.address)))
    });
    inviters
}

fn summarize_proxy_inviters(
    terminal_transfers: &[PathfindingTransferStep],
    owner_remap: &HashMap<Address, Address>,
    inviter: Address,
) -> Vec<ProxyInviter> {
    let mut totals = BTreeMap::<Address, U256>::new();

    for transfer in terminal_transfers {
        let Ok(raw_owner) = Address::from_str(&transfer.token_owner) else {
            continue;
        };
        let resolved_owner = owner_remap.get(&raw_owner).copied().unwrap_or(raw_owner);
        totals
            .entry(resolved_owner)
            .and_modify(|total| *total += transfer.value)
            .or_insert(transfer.value);
    }

    let fee = invitation_fee_amount();
    let inviters = totals
        .into_iter()
        .filter_map(|(address, amount)| {
            let possible = amount / fee;
            if possible == U256::ZERO {
                return None;
            }
            Some(ProxyInviter {
                address,
                possible_invites: u64::try_from(possible).unwrap_or(u64::MAX),
            })
        })
        .collect::<Vec<_>>();

    order_proxy_inviters(inviters, inviter)
}

impl HumanAvatar {
    pub(crate) async fn ensure_inviter_setup(&self) -> Result<Vec<PreparedTransaction>, SdkError> {
        let invitation_module = self.common.core.config.invitation_module_address;
        let module_enabled = SafeMinimal::new(self.address, self.core.provider())
            .isModuleEnabled(invitation_module)
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))?;

        let inviter_trusted = if module_enabled {
            self.core
                .hub_v2()
                .isTrusted(invitation_module, self.address)
                .call()
                .await
                .map_err(|e| SdkError::Contract(e.to_string()))?
        } else {
            false
        };

        Ok(build_inviter_setup_txs(
            self.address,
            invitation_module,
            module_enabled,
            inviter_trusted,
        ))
    }

    async fn plan_invitation_delivery(
        &self,
        tx_data: Bytes,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        let invitation_module = self.common.core.config.invitation_module_address;
        let mut transactions = self.ensure_inviter_setup().await?;
        let transfer_builder = TransferBuilder::new(self.common.core.config.clone())?;
        let proxy_inviters = self.proxy_inviters().await?;

        if let Some(proxy_inviter) = proxy_inviters.first() {
            let transfer_txs = transfer_builder
                .construct_advanced_transfer_with_aggregate(
                    self.address,
                    invitation_module,
                    invitation_fee_amount(),
                    Some(AdvancedTransferOptions {
                        use_wrapped_balances: Some(true),
                        from_tokens: None,
                        to_tokens: Some(vec![proxy_inviter.address]),
                        exclude_from_tokens: None,
                        exclude_to_tokens: None,
                        simulated_balances: None,
                        simulated_trusts: Some(vec![SimulatedTrust {
                            truster: invitation_module,
                            trustee: self.address,
                        }]),
                        max_transfers: None,
                        tx_data: Some(tx_data.clone()),
                    }),
                    true,
                )
                .await?;
            transactions.extend(transfer_txs_to_prepared(transfer_txs));
            return Ok(transactions);
        }

        let farm_txs = transfer_builder
            .construct_advanced_transfer_with_aggregate(
                self.address,
                farm_destination_address(),
                invitation_fee_amount(),
                Some(AdvancedTransferOptions {
                    use_wrapped_balances: Some(true),
                    from_tokens: None,
                    to_tokens: Some(vec![gnosis_group_address()]),
                    exclude_from_tokens: None,
                    exclude_to_tokens: None,
                    simulated_balances: None,
                    simulated_trusts: None,
                    max_transfers: None,
                    tx_data: None,
                }),
                true,
            )
            .await?;
        transactions.extend(transfer_txs_to_prepared(farm_txs));

        let claimed_id = self
            .core
            .invitation_farm()
            .claimInvite()
            .from(farm_quota_holder())
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))?;
        let live_invitation_module = self.invitation_module().await?;

        transactions.push(build_claim_invite_tx(
            self.common.core.config.invitation_farm_address,
        ));
        transactions.push(build_invitation_transfer_tx(
            self.common.core.config.v2_hub_address,
            self.address,
            live_invitation_module,
            claimed_id,
            tx_data,
        ));

        Ok(transactions)
    }

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

    #[cfg(feature = "ws")]
    pub async fn subscribe_events_ws_with_catchup(
        &self,
        ws_url: &str,
        filter: serde_json::Value,
        max_attempts: Option<usize>,
        catch_up_from_block: Option<u64>,
        catch_up_filter: Option<Vec<circles_types::Filter>>,
    ) -> Result<(Vec<CirclesEvent>, CirclesSubscription<CirclesEvent>), SdkError> {
        self.common
            .subscribe_events_ws_with_catchup(
                ws_url,
                filter,
                max_attempts,
                catch_up_from_block,
                catch_up_filter,
            )
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

    /// Compute the maximum amount that can be replenished into this human's own token.
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

    /// Find a path from this avatar to `to` for the requested target flow.
    pub async fn find_path(
        &self,
        to: Address,
        target_flow: U256,
        options: Option<AdvancedTransferOptions>,
    ) -> Result<PathfindingResult, SdkError> {
        self.common.find_path(to, target_flow, options).await
    }

    /// Compute the maximum available flow from this avatar to `to`.
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

    /// Get group memberships for this avatar using the shared paged-query helper.
    pub fn group_memberships(
        &self,
        limit: u32,
        sort_order: SortOrder,
    ) -> PagedQuery<GroupMembershipRow> {
        self.common
            .rpc
            .group()
            .get_group_memberships(self.address, limit, sort_order)
    }

    /// Fetch all group memberships, then resolve full group rows for the referenced groups.
    ///
    /// This mirrors the TS helper shape: `limit` is used as the page size for the
    /// membership query, not as a hard cap on the final enriched result count.
    pub async fn group_membership_details(&self, limit: u32) -> Result<Vec<GroupRow>, SdkError> {
        let mut query = self.group_memberships(limit, SortOrder::DESC);
        let mut memberships = Vec::new();

        while let Some(page) = query.next_page().await? {
            memberships.extend(page.items);
            if !page.has_more {
                break;
            }
        }

        if memberships.is_empty() {
            return Ok(Vec::new());
        }

        let group_addresses = memberships
            .into_iter()
            .map(|membership| membership.group)
            .collect::<Vec<_>>();

        Ok(self
            .common
            .rpc
            .group()
            .find_groups(
                group_addresses.len() as u32,
                Some(GroupQueryParams {
                    group_address_in: Some(group_addresses),
                    ..Default::default()
                }),
            )
            .await?)
    }

    /// Mint all currently claimable personal tokens (requires runner).
    pub async fn personal_mint(&self) -> Result<Vec<SubmittedTx>, SdkError> {
        let call = HubV2::personalMintCall {};
        let tx = call_to_tx(self.core.config.v2_hub_address, call, None);
        self.common.send(vec![tx]).await
    }

    /// Permanently stop personal token minting (requires runner).
    pub async fn stop_mint(&self) -> Result<Vec<SubmittedTx>, SdkError> {
        let call = HubV2::stopCall {};
        let tx = call_to_tx(self.core.config.v2_hub_address, call, None);
        self.common.send(vec![tx]).await
    }

    async fn generate_invites_inner(
        &self,
        number_of_invites: u64,
    ) -> Result<GeneratedInvites, SdkError> {
        if number_of_invites == 0 {
            return Err(SdkError::InvalidRegistration(
                "number_of_invites must be greater than 0".to_string(),
            ));
        }

        // Simulate claim to retrieve ids
        let claim_call = InvitationFarm::claimInvitesCall {
            numberOfInvites: U256::from(number_of_invites),
        };
        let ids = self
            .common
            .core
            .invitation_farm()
            .claimInvites(U256::from(number_of_invites))
            .call()
            .await
            .unwrap_or_default();
        if ids.is_empty() {
            return Err(SdkError::InvalidRegistration(
                "invitation farm returned no ids".to_string(),
            ));
        }

        // Secrets/signers
        let mut secrets = Vec::with_capacity(ids.len());
        let mut signers = Vec::with_capacity(ids.len());
        for _ in &ids {
            let secret = generate_private_key();
            let signer = private_key_to_address(&secret)?;
            secrets.push(secret);
            signers.push(signer);
        }

        // Referral payload
        let create_accounts = ReferralsModule::createAccountsCall {
            signers: signers.clone(),
        };
        let referrals_module = self.common.core.config.referrals_module_address;
        let payload = ReferralPayload {
            referralsModule: referrals_module,
            callData: create_accounts.abi_encode().into(),
        };
        let encoded_payload = payload.abi_encode();

        // Amounts: 96 CRC each
        let amount = invitation_fee_amount();
        let values = vec![amount; ids.len()];

        // Build txs: claimInvites + safeBatchTransferFrom to invitation module
        let invitation_module = self
            .common
            .core
            .invitation_farm()
            .invitationModule()
            .call()
            .await
            .unwrap_or_default();

        let claim_tx = call_to_tx(
            self.common.core.config.invitation_farm_address,
            claim_call,
            None,
        );
        let batch_call = HubV2::safeBatchTransferFromCall {
            _from: self.address,
            _to: invitation_module,
            _ids: ids,
            _values: values,
            _data: encoded_payload.into(),
        };
        let batch_tx = call_to_tx(self.common.core.config.v2_hub_address, batch_call, None);

        Ok(GeneratedInvites {
            secrets,
            signers,
            txs: vec![
                RunnerTx {
                    to: claim_tx.to,
                    data: claim_tx.data,
                    value: claim_tx.value,
                },
                RunnerTx {
                    to: batch_tx.to,
                    data: batch_tx.data,
                    value: batch_tx.value,
                },
            ],
            submitted: None,
        })
    }

    /// Plan batch referral generation via the invitation farm without submitting.
    pub async fn plan_generate_referrals(
        &self,
        number_of_invites: u64,
    ) -> Result<GeneratedInvites, SdkError> {
        self.generate_invites_inner(number_of_invites).await
    }

    /// Backward-compatible alias for the older plan-only helper name.
    pub async fn generate_invites(
        &self,
        number_of_invites: u64,
    ) -> Result<GeneratedInvites, SdkError> {
        self.plan_generate_referrals(number_of_invites).await
    }

    /// Execute batch referral generation using the configured runner.
    pub async fn generate_referrals(
        &self,
        number_of_invites: u64,
    ) -> Result<GeneratedInvites, SdkError> {
        let generated = self.plan_generate_referrals(number_of_invites).await?;
        self.submit_generated_referrals(generated).await
    }

    /// Invitation fee in atto-circles (96 CRC), matching the TS helper constant.
    pub fn invitation_fee(&self) -> U256 {
        invitation_fee_amount()
    }

    /// Invitation module address currently configured on the invitation farm.
    pub async fn invitation_module(&self) -> Result<Address, SdkError> {
        self.common
            .core
            .invitation_farm()
            .invitationModule()
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
    }

    /// Remaining invitation quota available to this avatar on the invitation farm.
    pub async fn invitation_quota(&self) -> Result<U256, SdkError> {
        self.common
            .core
            .invitation_farm()
            .inviterQuota(self.address)
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
    }

    /// Compute the deterministic Safe address used by the referrals module for a signer.
    pub async fn compute_referral_address(&self, signer: Address) -> Result<Address, SdkError> {
        self.common
            .core
            .referrals_module()
            .computeAddress(signer)
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
    }

    /// Unified invitation-origin helper for this avatar.
    pub async fn invitation_origin(&self) -> Result<Option<InvitationOriginResponse>, SdkError> {
        Ok(self
            .common
            .rpc
            .invitation()
            .get_invitation_origin(self.address)
            .await?)
    }

    /// Direct inviter for this avatar when available.
    pub async fn invited_by(&self) -> Result<Option<Address>, SdkError> {
        Ok(self
            .common
            .rpc
            .invitation()
            .get_invited_by(self.address)
            .await?)
    }

    /// Combined invitation availability for this avatar across trust, escrow, and at-scale sources.
    pub async fn available_invitations(
        &self,
        minimum_balance: Option<String>,
    ) -> Result<AllInvitationsResponse, SdkError> {
        Ok(self
            .common
            .rpc
            .invitation()
            .get_all_invitations(self.address, minimum_balance)
            .await?)
    }

    /// Trust-based invitation availability for this avatar.
    pub async fn trust_invitations(
        &self,
        minimum_balance: Option<String>,
    ) -> Result<Vec<TrustInvitation>, SdkError> {
        Ok(self
            .common
            .rpc
            .invitation()
            .get_trust_invitations(self.address, minimum_balance)
            .await?)
    }

    /// Escrow-backed invitation availability for this avatar.
    pub async fn escrow_invitations(&self) -> Result<Vec<EscrowInvitation>, SdkError> {
        Ok(self
            .common
            .rpc
            .invitation()
            .get_escrow_invitations(self.address)
            .await?)
    }

    /// At-scale invitation availability for this avatar.
    pub async fn at_scale_invitations(&self) -> Result<Vec<AtScaleInvitation>, SdkError> {
        Ok(self
            .common
            .rpc
            .invitation()
            .get_at_scale_invitations(self.address)
            .await?)
    }

    /// Proxy inviters that can route invitations to the invitation module.
    pub async fn proxy_inviters(&self) -> Result<Vec<ProxyInviter>, SdkError> {
        let invitation_module = self.common.core.config.invitation_module_address;

        let gnosis_group_trusts = self
            .common
            .rpc
            .trust()
            .get_trusts(gnosis_group_address())
            .await?;
        let trusts_inviter_relations = self.common.rpc.trust().get_trusted_by(self.address).await?;
        let mutual_trust_relations = self
            .common
            .rpc
            .trust()
            .get_mutual_trusts(self.address)
            .await?;
        let module_trusts_relations = self
            .common
            .rpc
            .trust()
            .get_trusts(invitation_module)
            .await?;
        let module_mutual_trust_relations = self
            .common
            .rpc
            .trust()
            .get_mutual_trusts(invitation_module)
            .await?;

        let set1 = gnosis_group_trusts
            .into_iter()
            .map(|relation| relation.object_avatar)
            .collect::<std::collections::HashSet<_>>();
        let set2 = trusts_inviter_relations
            .into_iter()
            .chain(mutual_trust_relations.into_iter())
            .map(|relation| relation.object_avatar)
            .collect::<std::collections::HashSet<_>>();
        let set3 = module_trusts_relations
            .into_iter()
            .chain(module_mutual_trust_relations.into_iter())
            .map(|relation| relation.object_avatar)
            .collect::<std::collections::HashSet<_>>();

        let mut tokens_to_use = set2
            .into_iter()
            .filter(|address| set3.contains(address) && !set1.contains(address))
            .collect::<Vec<_>>();
        tokens_to_use.push(self.address);

        let path = self
            .common
            .find_path(
                invitation_module,
                invitation_max_flow(),
                Some(AdvancedTransferOptions {
                    use_wrapped_balances: Some(true),
                    from_tokens: None,
                    to_tokens: Some(tokens_to_use),
                    exclude_from_tokens: None,
                    exclude_to_tokens: None,
                    simulated_balances: None,
                    simulated_trusts: Some(vec![SimulatedTrust {
                        truster: invitation_module,
                        trustee: self.address,
                    }]),
                    max_transfers: None,
                    tx_data: None,
                }),
            )
            .await?;

        if path.transfers.is_empty() {
            return Ok(Vec::new());
        }

        let terminal_transfers = path
            .transfers
            .iter()
            .filter(|transfer| transfer.to == invitation_module)
            .cloned()
            .collect::<Vec<_>>();
        if terminal_transfers.is_empty() {
            return Ok(Vec::new());
        }

        let raw_owners = terminal_transfers
            .iter()
            .filter_map(|transfer| Address::from_str(&transfer.token_owner).ok())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        let token_infos = self
            .common
            .rpc
            .token_info()
            .get_token_info_batch(raw_owners)
            .await?;
        let owner_remap = token_infos
            .into_iter()
            .map(|info| (info.token, info.token_owner))
            .collect::<HashMap<_, _>>();

        Ok(summarize_proxy_inviters(
            &terminal_transfers,
            &owner_remap,
            self.address,
        ))
    }

    /// Find an invitation path to the configured invitation module, optionally forcing a proxy inviter.
    pub async fn find_invite_path(
        &self,
        proxy_inviter_address: Option<Address>,
    ) -> Result<PathfindingResult, SdkError> {
        let invitation_module = self.common.core.config.invitation_module_address;
        let token_to_use = match proxy_inviter_address {
            Some(address) => address,
            None => self
                .proxy_inviters()
                .await?
                .into_iter()
                .next()
                .map(|inviter| inviter.address)
                .ok_or_else(|| {
                    SdkError::OperationFailed(format!(
                        "no proxy inviters available for {:#x}",
                        self.address
                    ))
                })?,
        };

        let path = self
            .common
            .find_path(
                invitation_module,
                invitation_fee_amount(),
                Some(AdvancedTransferOptions {
                    use_wrapped_balances: Some(true),
                    from_tokens: None,
                    to_tokens: Some(vec![token_to_use]),
                    exclude_from_tokens: None,
                    exclude_to_tokens: None,
                    simulated_balances: None,
                    simulated_trusts: Some(vec![SimulatedTrust {
                        truster: invitation_module,
                        trustee: self.address,
                    }]),
                    max_transfers: None,
                    tx_data: None,
                }),
            )
            .await?;

        if path.transfers.is_empty() {
            return Err(SdkError::OperationFailed(format!(
                "no invitation path found from {:#x} to {:#x}",
                self.address, invitation_module
            )));
        }

        if path.max_flow < invitation_fee_amount() {
            return Err(SdkError::OperationFailed(format!(
                "insufficient balance for invitation flow from {:#x}: requested {} wei, available {} wei",
                self.address,
                invitation_fee_amount(),
                path.max_flow
            )));
        }

        Ok(path)
    }

    /// Find a fallback path from this avatar to the invitation farm destination.
    pub async fn find_farm_invite_path(&self) -> Result<PathfindingResult, SdkError> {
        let farm_destination = farm_destination_address();
        let path = self
            .common
            .find_path(
                farm_destination,
                invitation_fee_amount(),
                Some(AdvancedTransferOptions {
                    use_wrapped_balances: Some(true),
                    from_tokens: None,
                    to_tokens: Some(vec![gnosis_group_address()]),
                    exclude_from_tokens: None,
                    exclude_to_tokens: None,
                    simulated_balances: None,
                    simulated_trusts: None,
                    max_transfers: None,
                    tx_data: None,
                }),
            )
            .await?;

        if path.transfers.is_empty() {
            return Err(SdkError::OperationFailed(format!(
                "no invitation farm path found from {:#x} to {:#x}",
                self.address, farm_destination
            )));
        }

        if path.max_flow < invitation_fee_amount() {
            return Err(SdkError::OperationFailed(format!(
                "insufficient balance for invitation farm flow from {:#x}: requested {} wei, available {} wei",
                self.address,
                invitation_fee_amount(),
                path.max_flow
            )));
        }

        Ok(path)
    }

    /// Plan a direct invite flow for an existing Safe wallet without submitting.
    pub async fn plan_invite(
        &self,
        invitee: Address,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        let is_human = self
            .core
            .hub_v2()
            .isHuman(invitee)
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))?;
        if is_human {
            return Err(SdkError::OperationFailed(format!(
                "Invitee {invitee:#x} is already registered as a human in Circles Hub. Cannot invite an already registered user."
            )));
        }

        self.plan_invitation_delivery(encode_direct_invite_data(invitee))
            .await
    }

    /// Execute a direct invite flow using the configured runner.
    pub async fn invite(&self, invitee: Address) -> Result<Vec<SubmittedTx>, SdkError> {
        let txs = self.plan_invite(invitee).await?;
        self.common.send(txs).await
    }

    /// Plan the TS-style single-referral flow and return the generated private key.
    pub async fn plan_referral_code(&self) -> Result<ReferralCodePlan, SdkError> {
        let private_key = generate_private_key();
        let signer = private_key_to_address(&private_key)?;
        let txs = self
            .plan_invitation_delivery(encode_referral_invite_data(
                signer,
                self.common.core.config.referrals_module_address,
            ))
            .await?;

        Ok(ReferralCodePlan {
            private_key,
            signer,
            txs,
        })
    }

    /// Backward-compatible TS-style name for the single-referral planner.
    pub async fn get_referral_code(&self) -> Result<ReferralCodePlan, SdkError> {
        self.plan_referral_code().await
    }

    /// Accounts invited by this avatar, filtered by accepted vs pending status.
    pub async fn invitations_from(
        &self,
        accepted: bool,
    ) -> Result<InvitationsFromResponse, SdkError> {
        Ok(self
            .common
            .rpc
            .invitation()
            .get_invitations_from(self.address, accepted)
            .await?)
    }

    /// Accepted invitees for this avatar.
    pub async fn accepted_invitees(&self) -> Result<Vec<InvitedAccountInfo>, SdkError> {
        Ok(self.invitations_from(true).await?.results)
    }

    /// Pending invitees for this avatar.
    pub async fn pending_invitees(&self) -> Result<Vec<InvitedAccountInfo>, SdkError> {
        Ok(self.invitations_from(false).await?.results)
    }

    /// Public referral previews created by this avatar via the optional referrals backend.
    pub async fn list_referrals(
        &self,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<ReferralPreviewList, SdkError> {
        let referrals_service_url = self
            .common
            .core
            .config
            .referrals_service_url
            .as_deref()
            .ok_or_else(|| {
                SdkError::OperationFailed(
                    "Referrals service not configured. Set referrals_service_url in CirclesConfig."
                        .to_string(),
                )
            })?;
        let referrals = Referrals::new(referrals_service_url, self.core.clone())?;

        Ok(referrals
            .list_public(
                self.address,
                Some(ReferralPublicListOptions {
                    limit,
                    offset,
                    in_session: None,
                }),
            )
            .await?)
    }

    async fn submit_generated_referrals(
        &self,
        mut generated: GeneratedInvites,
    ) -> Result<GeneratedInvites, SdkError> {
        generated.submitted = Some(self.common.send(generated.txs.clone()).await?);
        Ok(generated)
    }

    /// Legacy invitation-balance rows (RPC helper).
    pub async fn invitations(
        &self,
    ) -> Result<Vec<circles_rpc::methods::invitation::InvitationRow>, SdkError> {
        Ok(self
            .common
            .rpc
            .invitation()
            .get_invitations(self.address)
            .await?)
    }

    /// Redeem an invitation from an inviter (requires runner).
    pub async fn redeem_invitation(&self, inviter: Address) -> Result<Vec<SubmittedTx>, SdkError> {
        let call = circles_abis::InvitationEscrow::redeemInvitationCall { inviter };
        let tx = call_to_tx(
            self.common.core.config.invitation_escrow_address,
            call,
            None,
        );
        self.common.send(vec![tx]).await
    }

    /// Revoke a specific invitation (requires runner).
    pub async fn revoke_invitation(&self, invitee: Address) -> Result<Vec<SubmittedTx>, SdkError> {
        let call = circles_abis::InvitationEscrow::revokeInvitationCall { invitee };
        let tx = call_to_tx(
            self.common.core.config.invitation_escrow_address,
            call,
            None,
        );
        self.common.send(vec![tx]).await
    }

    /// Revoke all invitations sent by this avatar (requires runner).
    pub async fn revoke_all_invitations(&self) -> Result<Vec<SubmittedTx>, SdkError> {
        let call = circles_abis::InvitationEscrow::revokeAllInvitationsCall {};
        let tx = call_to_tx(
            self.common.core.config.invitation_escrow_address,
            call,
            None,
        );
        self.common.send(vec![tx]).await
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Bytes, TxHash, address};
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
            chain_rpc_url: None,
            pathfinder_url: Some("https://pathfinder.example.com".into()),
            profile_service_url: Some("https://profiles.example.com".into()),
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
            avatar_type: AvatarType::CrcV2RegisterHuman,
            avatar: address,
            token_id: None,
            has_v1: false,
            v1_token: None,
            cid_v0_digest: None,
            cid_v0: None,
            v1_stopped: None,
            is_human: true,
            name: None,
            symbol: None,
        }
    }

    fn test_avatar() -> (HumanAvatar, Arc<RecordingRunner>, CirclesConfig) {
        let config = dummy_config();
        let runner = Arc::new(RecordingRunner {
            sender: Address::repeat_byte(0xaa),
            sent: Mutex::new(Vec::new()),
        });
        let avatar = HumanAvatar::new(
            Address::repeat_byte(0xaa),
            dummy_avatar(Address::repeat_byte(0xaa)),
            Arc::new(Core::new(config.clone())),
            Profiles::new(config.effective_profile_service_url()).expect("profiles"),
            Arc::new(CirclesRpc::try_from_http(&config.circles_rpc_url).expect("rpc")),
            Some(runner.clone()),
        );
        (avatar, runner, config)
    }

    #[test]
    fn referral_payload_encodes() {
        let signers = vec![address!("1000000000000000000000000000000000000001")];
        let create_accounts = ReferralsModule::createAccountsCall {
            signers: signers.clone(),
        };
        let payload = ReferralPayload {
            referralsModule: address!("2000000000000000000000000000000000000002"),
            callData: create_accounts.abi_encode().into(),
        };
        let bytes = payload.abi_encode();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn batch_tx_targets_hub() {
        let ids = vec![U256::from(1), U256::from(2)];
        let amount = U256::from(96u128) * U256::from(10).pow(U256::from(18));
        let values = vec![amount; ids.len()];
        let batch_call = HubV2::safeBatchTransferFromCall {
            _from: address!("aaaa000000000000000000000000000000000000"),
            _to: address!("bbbb000000000000000000000000000000000000"),
            _ids: ids,
            _values: values,
            _data: Bytes::default(),
        };
        let batch_tx = call_to_tx(
            address!("cccc000000000000000000000000000000000000"),
            batch_call,
            None,
        );
        assert_eq!(
            batch_tx.to,
            address!("cccc000000000000000000000000000000000000")
        );
    }

    #[test]
    fn invitation_fee_matches_ts_constant() {
        assert_eq!(
            invitation_fee_amount(),
            U256::from(96u128) * U256::from(10).pow(U256::from(18))
        );
    }

    #[test]
    fn order_proxy_inviters_prioritizes_inviter() {
        let inviter = Address::repeat_byte(0xaa);
        let ordered = order_proxy_inviters(
            vec![
                ProxyInviter {
                    address: Address::repeat_byte(0xbb),
                    possible_invites: 1,
                },
                ProxyInviter {
                    address: inviter,
                    possible_invites: 2,
                },
            ],
            inviter,
        );

        assert_eq!(ordered[0].address, inviter);
    }

    #[test]
    fn summarize_proxy_inviters_rewrites_wrapped_owners() {
        let inviter = Address::repeat_byte(0xaa);
        let wrapper = Address::repeat_byte(0xbb);
        let other = Address::repeat_byte(0xcc);
        let terminal = vec![
            PathfindingTransferStep {
                from: Address::repeat_byte(0x01),
                to: Address::repeat_byte(0x02),
                token_owner: format!("{wrapper:#x}"),
                value: invitation_fee_amount() * U256::from(2u64),
            },
            PathfindingTransferStep {
                from: Address::repeat_byte(0x03),
                to: Address::repeat_byte(0x02),
                token_owner: format!("{other:#x}"),
                value: invitation_fee_amount(),
            },
        ];
        let owner_remap = HashMap::from([(wrapper, inviter)]);

        let inviters = summarize_proxy_inviters(&terminal, &owner_remap, inviter);

        assert_eq!(inviters.len(), 2);
        assert_eq!(inviters[0].address, inviter);
        assert_eq!(inviters[0].possible_invites, 2);
        assert_eq!(inviters[1].address, other);
        assert_eq!(inviters[1].possible_invites, 1);
    }

    #[test]
    fn direct_invite_data_encodes_single_address() {
        let invitee = address!("1234000000000000000000000000000000000000");

        assert_eq!(
            encode_direct_invite_data(invitee),
            Bytes::from(invitee.abi_encode())
        );
    }

    #[test]
    fn farm_constants_match_ts_values() {
        assert_eq!(
            farm_destination_address(),
            address!("9Eb51E6A39B3F17bB1883B80748b56170039ff1d")
        );
        assert_eq!(
            farm_quota_holder(),
            address!("20EcD8bDeb2F48d8a7c94E542aA4feC5790D9676")
        );
    }

    #[test]
    fn build_inviter_setup_txs_enable_then_trust_when_module_missing() {
        let inviter = address!("1000000000000000000000000000000000000001");
        let invitation_module = address!("2000000000000000000000000000000000000002");
        let txs = build_inviter_setup_txs(inviter, invitation_module, false, false);

        assert_eq!(txs.len(), 2);
        assert_eq!(txs[0].to, inviter);
        assert_eq!(
            &txs[0].data[..4],
            &SafeMinimal::enableModuleCall {
                module: invitation_module,
            }
            .abi_encode()[..4]
        );
        assert_eq!(txs[1].to, invitation_module);
        assert_eq!(
            &txs[1].data[..4],
            &InvitationModuleMinimal::trustInviterCall { inviter }.abi_encode()[..4]
        );
    }

    #[test]
    fn build_inviter_setup_txs_only_trust_when_module_enabled() {
        let inviter = address!("3000000000000000000000000000000000000003");
        let invitation_module = address!("4000000000000000000000000000000000000004");
        let txs = build_inviter_setup_txs(inviter, invitation_module, true, false);

        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].to, invitation_module);
        assert_eq!(
            &txs[0].data[..4],
            &InvitationModuleMinimal::trustInviterCall { inviter }.abi_encode()[..4]
        );
    }

    #[test]
    fn build_inviter_setup_txs_skip_when_already_ready() {
        let txs = build_inviter_setup_txs(
            address!("5000000000000000000000000000000000000005"),
            address!("6000000000000000000000000000000000000006"),
            true,
            true,
        );

        assert!(txs.is_empty());
    }

    #[test]
    fn claim_invite_tx_targets_farm() {
        let farm = address!("7000000000000000000000000000000000000007");
        let tx = build_claim_invite_tx(farm);

        assert_eq!(tx.to, farm);
        assert_eq!(
            &tx.data[..4],
            &InvitationFarm::claimInviteCall {}.abi_encode()[..4]
        );
    }

    #[test]
    fn direct_invite_transfer_tx_matches_hub_call() {
        let tx = build_invitation_transfer_tx(
            address!("8000000000000000000000000000000000000008"),
            address!("9000000000000000000000000000000000000009"),
            address!("a00000000000000000000000000000000000000a"),
            U256::from(123u64),
            encode_direct_invite_data(address!("b00000000000000000000000000000000000000b")),
        );

        let expected = HubV2::safeTransferFromCall {
            _from: address!("9000000000000000000000000000000000000009"),
            _to: address!("a00000000000000000000000000000000000000a"),
            _id: U256::from(123u64),
            _value: invitation_fee_amount(),
            _data: encode_direct_invite_data(address!("b00000000000000000000000000000000000000b")),
        };

        assert_eq!(tx.to, address!("8000000000000000000000000000000000000008"));
        assert_eq!(tx.data, Bytes::from(expected.abi_encode()));
        assert_eq!(tx.value, None);
    }

    #[test]
    fn referral_invite_data_encodes_create_account_payload() {
        let signer = address!("c00000000000000000000000000000000000000c");
        let referrals_module = address!("d00000000000000000000000000000000000000d");
        let data = encode_referral_invite_data(signer, referrals_module);

        let expected_create_account = ReferralsModule::createAccountCall { signer };
        let expected_payload = ReferralPayload {
            referralsModule: referrals_module,
            callData: expected_create_account.abi_encode().into(),
        };

        assert_eq!(data, Bytes::from(expected_payload.abi_encode()));
    }

    #[tokio::test]
    async fn write_helpers_encode_expected_calls() {
        let (avatar, runner, config) = test_avatar();

        avatar
            .update_profile_metadata(TEST_CID)
            .await
            .expect("update metadata");
        avatar
            .register_short_name(7)
            .await
            .expect("register short name");
        avatar.personal_mint().await.expect("personal mint");
        avatar.stop_mint().await.expect("stop mint");

        let sent = runner.sent.lock().expect("lock");
        assert_eq!(sent.len(), 4);

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
                _nonce: U256::from(7u64),
            }
            .abi_encode()[..4]
        );

        assert_eq!(sent[2][0].to, config.v2_hub_address);
        assert_eq!(
            &sent[2][0].data[..4],
            &HubV2::personalMintCall {}.abi_encode()[..4]
        );

        assert_eq!(sent[3][0].to, config.v2_hub_address);
        assert_eq!(&sent[3][0].data[..4], &HubV2::stopCall {}.abi_encode()[..4]);
    }

    #[tokio::test]
    async fn submit_generated_referrals_uses_runner_for_prepared_batch() {
        let (avatar, runner, _config) = test_avatar();
        let prepared = GeneratedInvites {
            secrets: vec!["0x1234".into()],
            signers: vec![Address::repeat_byte(0x55)],
            txs: vec![
                RunnerTx {
                    to: Address::repeat_byte(0x11),
                    data: Bytes::from(vec![0xaa]),
                    value: None,
                },
                RunnerTx {
                    to: Address::repeat_byte(0x22),
                    data: Bytes::from(vec![0xbb]),
                    value: None,
                },
            ],
            submitted: None,
        };

        let result = avatar
            .submit_generated_referrals(prepared)
            .await
            .expect("submit generated referrals");

        assert_eq!(result.txs.len(), 2);
        assert!(result.submitted.is_some());

        let sent = runner.sent.lock().expect("lock");
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].len(), 2);
    }
}
