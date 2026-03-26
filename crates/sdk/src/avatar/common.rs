#[cfg(feature = "ws")]
use crate::ws;
use crate::{ContractRunner, Core, PreparedTransaction, Profile, SdkError};
use alloy_primitives::{Address, U256};
use circles_profiles::Profiles;
#[cfg(feature = "ws")]
use circles_rpc::events::subscription::CirclesSubscription;
use circles_rpc::{CirclesRpc, PagedQuery};
use circles_transfers::TransferBuilder;
use circles_types::{
    AdvancedTransferOptions, AggregatedTrustRelation, Balance, PathfindingResult, SortOrder,
    TokenBalanceResponse, TransactionHistoryRow, TrustRelation, TrustRelationType,
};
#[cfg(feature = "ws")]
use circles_types::{CirclesEvent, Filter};
#[cfg(feature = "ws")]
use serde_json::json;
use std::sync::Arc;

/// Shared avatar context and read helpers.
///
/// Most methods are read-only; ones that submit transactions require a runner
/// and return `SdkError::MissingRunner` if absent.
pub struct CommonAvatar {
    pub address: Address,
    pub core: Arc<Core>,
    pub profiles: Profiles,
    pub rpc: Arc<CirclesRpc>,
    pub runner: Option<Arc<dyn ContractRunner>>,
}

impl CommonAvatar {
    pub fn new(
        address: Address,
        core: Arc<Core>,
        profiles: Profiles,
        rpc: Arc<CirclesRpc>,
        runner: Option<Arc<dyn ContractRunner>>,
    ) -> Self {
        Self {
            address,
            core,
            profiles,
            rpc,
            runner,
        }
    }

    /// Get detailed token balances (v1/v2 selectable).
    pub async fn balances(
        &self,
        as_time_circles: bool,
        use_v2: bool,
    ) -> Result<Vec<TokenBalanceResponse>, SdkError> {
        Ok(self
            .rpc
            .token()
            .get_token_balances(self.address, as_time_circles, use_v2)
            .await?)
    }

    /// Get aggregate balance (v1/v2 selectable).
    pub async fn total_balance(
        &self,
        as_time_circles: bool,
        use_v2: bool,
    ) -> Result<Balance, SdkError> {
        Ok(self
            .rpc
            .balance()
            .get_total_balance(self.address, as_time_circles, use_v2)
            .await?)
    }

    /// Get trust relations.
    pub async fn trust_relations(&self) -> Result<Vec<TrustRelation>, SdkError> {
        Ok(self.rpc.trust().get_trust_relations(self.address).await?)
    }

    /// Get aggregated trust relations, matching the TS SDK convenience surface.
    pub async fn aggregated_trust_relations(
        &self,
    ) -> Result<Vec<AggregatedTrustRelation>, SdkError> {
        Ok(self
            .rpc
            .trust()
            .get_aggregated_trust_relations(self.address)
            .await?)
    }

    /// Get outgoing trust relations only.
    pub async fn trusts(&self) -> Result<Vec<AggregatedTrustRelation>, SdkError> {
        Ok(self.rpc.trust().get_trusts(self.address).await?)
    }

    /// Get incoming trust relations only.
    pub async fn trusted_by(&self) -> Result<Vec<AggregatedTrustRelation>, SdkError> {
        Ok(self.rpc.trust().get_trusted_by(self.address).await?)
    }

    /// Get mutual trust relations only.
    pub async fn mutual_trusts(&self) -> Result<Vec<AggregatedTrustRelation>, SdkError> {
        Ok(self.rpc.trust().get_mutual_trusts(self.address).await?)
    }

    /// Check whether this avatar trusts `other_avatar`.
    pub async fn is_trusting(&self, other_avatar: Address) -> Result<bool, SdkError> {
        let rels = self
            .rpc
            .trust()
            .get_common_trust(self.address, other_avatar)
            .await?;
        Ok(rels.iter().any(|rel| {
            matches!(
                rel,
                TrustRelationType::Trusts | TrustRelationType::MutuallyTrusts
            )
        }))
    }

    /// Check whether `other_avatar` trusts this avatar.
    pub async fn is_trusted_by(&self, other_avatar: Address) -> Result<bool, SdkError> {
        let rels = self
            .rpc
            .trust()
            .get_common_trust(self.address, other_avatar)
            .await?;
        Ok(rels.iter().any(|rel| {
            matches!(
                rel,
                TrustRelationType::TrustedBy | TrustRelationType::MutuallyTrusts
            )
        }))
    }

    /// Fetch profile by CID if present.
    pub async fn profile(&self, cid: Option<&str>) -> Result<Option<Profile>, SdkError> {
        if let Some(cid) = cid {
            Ok(self.profiles.get(cid).await?)
        } else {
            Ok(None)
        }
    }

    /// Get transaction history for this avatar using the shared RPC paged query.
    pub fn transaction_history(
        &self,
        limit: u32,
        sort_order: SortOrder,
    ) -> PagedQuery<TransactionHistoryRow> {
        self.rpc
            .transaction()
            .get_transaction_history(self.address, limit, sort_order)
    }

    /// Upload profile to the profile service, returning the new CID.
    pub async fn pin_profile(&self, profile: &Profile) -> Result<String, SdkError> {
        Ok(self.profiles.create(profile).await?)
    }

    /// Submit transactions via runner (helper).
    pub async fn send(
        &self,
        txs: Vec<PreparedTransaction>,
    ) -> Result<Vec<crate::SubmittedTx>, SdkError> {
        let runner = self.runner.clone().ok_or(SdkError::MissingRunner)?;
        Ok(runner.send_transactions(txs).await?)
    }

    /// Plan a transfer using the transfer builder (no submit). Returns ordered prepared txs.
    ///
    /// Wrapper handling matches the TS SDK: unwrap inflationary/demurraged as
    /// needed and include re-wrap when static balances are present.
    pub async fn plan_transfer(
        &self,
        to: Address,
        amount: U256,
        options: Option<AdvancedTransferOptions>,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        let builder = TransferBuilder::new(self.core.config.clone())?;
        let txs = builder
            .construct_advanced_transfer(self.address, to, amount, options)
            .await?;
        Ok(txs
            .into_iter()
            .map(|tx| PreparedTransaction {
                to: tx.to,
                data: tx.data,
                value: Some(tx.value),
            })
            .collect())
    }

    /// Plan and execute a transfer using the runner (if present).
    pub async fn transfer(
        &self,
        to: Address,
        amount: U256,
        options: Option<AdvancedTransferOptions>,
    ) -> Result<Vec<crate::SubmittedTx>, SdkError> {
        let txs = self.plan_transfer(to, amount, options).await?;
        self.send(txs).await
    }

    /// Plan a replenish flow for `token_id`, optionally delivering the final
    /// balance to `receiver` instead of keeping it on this avatar.
    pub async fn plan_replenish(
        &self,
        token_id: Address,
        amount: U256,
        receiver: Option<Address>,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        let builder = TransferBuilder::new(self.core.config.clone())?;
        let txs = builder
            .construct_replenish(self.address, token_id, amount, receiver)
            .await?;
        Ok(txs
            .into_iter()
            .map(|tx| PreparedTransaction {
                to: tx.to,
                data: tx.data,
                value: Some(tx.value),
            })
            .collect())
    }

    /// Plan and execute a replenish flow using the runner (if present).
    pub async fn replenish(
        &self,
        token_id: Address,
        amount: U256,
        receiver: Option<Address>,
    ) -> Result<Vec<crate::SubmittedTx>, SdkError> {
        let txs = self.plan_replenish(token_id, amount, receiver).await?;
        self.send(txs).await
    }

    /// Find a path between this avatar and `to` with a target flow (defaults use_wrapped_balances=true).
    pub async fn find_path(
        &self,
        to: Address,
        target_flow: U256,
        options: Option<AdvancedTransferOptions>,
    ) -> Result<PathfindingResult, SdkError> {
        let opts = options.unwrap_or_else(|| AdvancedTransferOptions {
            use_wrapped_balances: Some(true),
            from_tokens: None,
            to_tokens: None,
            exclude_from_tokens: None,
            exclude_to_tokens: None,
            simulated_balances: None,
            simulated_trusts: None,
            max_transfers: None,
            tx_data: None,
        });
        let params = opts.to_find_path_params(self.address, to, target_flow);
        Ok(self.rpc.pathfinder().find_path(params).await?)
    }

    /// Max-flow helper: sets target_flow to U256::MAX.
    pub async fn max_flow_to(
        &self,
        to: Address,
        options: Option<AdvancedTransferOptions>,
    ) -> Result<PathfindingResult, SdkError> {
        self.find_path(to, U256::MAX, options).await
    }

    /// Subscribe to Circles events for this avatar via websocket.
    #[cfg(feature = "ws")]
    pub async fn subscribe_events_ws(
        &self,
        ws_url: &str,
        filter: Option<serde_json::Value>,
    ) -> Result<CirclesSubscription<CirclesEvent>, SdkError> {
        let filt = filter.unwrap_or_else(|| json!({ "address": format!("{:#x}", self.address) }));
        ws::subscribe_with_retries(ws_url, filt, None).await
    }

    #[cfg(feature = "ws")]
    pub async fn subscribe_events_ws_with_retries(
        &self,
        ws_url: &str,
        filter: serde_json::Value,
        max_attempts: Option<usize>,
    ) -> Result<CirclesSubscription<CirclesEvent>, SdkError> {
        ws::subscribe_with_retries(ws_url, filter, max_attempts).await
    }

    /// Subscribe with retries and optionally fetch HTTP events for a catch-up range.
    /// Returns (catch_up_events, live_subscription).
    #[cfg(feature = "ws")]
    pub async fn subscribe_events_ws_with_catchup(
        &self,
        ws_url: &str,
        filter: serde_json::Value,
        max_attempts: Option<usize>,
        catch_up_from_block: Option<u64>,
        catch_up_filter: Option<Vec<Filter>>,
    ) -> Result<(Vec<CirclesEvent>, CirclesSubscription<CirclesEvent>), SdkError> {
        ws::subscribe_with_catchup(
            &self.rpc,
            ws_url,
            filter,
            max_attempts,
            catch_up_from_block,
            catch_up_filter,
            Some(self.address),
        )
        .await
    }
}
