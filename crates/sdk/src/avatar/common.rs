#[cfg(feature = "ws")]
use crate::ws;
use crate::{ContractRunner, Core, PreparedTransaction, Profile, SdkError, call_to_tx};
use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::sol;
use circles_abis::{DemurrageCircles, HubV2, InflationaryCircles};
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

sol! {
    interface IERC20Like {
        function transfer(address to, uint256 value) external returns (bool);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WrapperKind {
    Demurrage,
    Inflation,
}

impl WrapperKind {
    fn hub_type(self) -> u8 {
        match self {
            Self::Demurrage => 0,
            Self::Inflation => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirectTransferKind {
    Erc1155,
    Erc20,
}

fn classify_direct_transfer_kind(token_type: &str) -> Option<DirectTransferKind> {
    match token_type {
        "CrcV2_RegisterHuman" | "CrcV2_RegisterGroup" => Some(DirectTransferKind::Erc1155),
        "CrcV2_ERC20WrapperDeployed_Demurraged" | "CrcV2_ERC20WrapperDeployed_Inflationary" => {
            Some(DirectTransferKind::Erc20)
        }
        _ => None,
    }
}

fn build_direct_erc1155_transfer_tx(
    hub: Address,
    from: Address,
    to: Address,
    token_id: U256,
    amount: U256,
    data: Bytes,
) -> PreparedTransaction {
    let call = HubV2::safeTransferFromCall {
        _from: from,
        _to: to,
        _id: token_id,
        _value: amount,
        _data: data,
    };
    call_to_tx(hub, call, None)
}

fn address_to_erc1155_token_id(token: Address) -> U256 {
    U256::from_be_slice(token.as_slice())
}

fn build_direct_erc20_transfer_tx(
    token: Address,
    to: Address,
    amount: U256,
) -> PreparedTransaction {
    let call = IERC20Like::transferCall { to, value: amount };
    call_to_tx(token, call, None)
}

fn build_wrap_erc20_tx(
    hub: Address,
    avatar: Address,
    amount: U256,
    kind: WrapperKind,
) -> PreparedTransaction {
    let call = HubV2::wrapCall {
        _avatar: avatar,
        _amount: amount,
        _type: kind.hub_type(),
    };
    call_to_tx(hub, call, None)
}

fn build_unwrap_demurrage_erc20_tx(wrapper: Address, amount: U256) -> PreparedTransaction {
    let call = DemurrageCircles::unwrapCall { _amount: amount };
    call_to_tx(wrapper, call, None)
}

fn build_unwrap_inflation_erc20_tx(wrapper: Address, amount: U256) -> PreparedTransaction {
    let call = InflationaryCircles::unwrapCall { _amount: amount };
    call_to_tx(wrapper, call, None)
}

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

    /// Plan a direct transfer without pathfinding.
    ///
    /// Mirrors the TS direct-transfer helper:
    /// - human/group tokens use `HubV2.safeTransferFrom`
    /// - wrapped ERC20 tokens use `transfer(address,uint256)`
    pub async fn plan_direct_transfer(
        &self,
        to: Address,
        amount: U256,
        token_address: Option<Address>,
        tx_data: Option<Bytes>,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        if amount.is_zero() {
            return Err(SdkError::OperationFailed(
                "direct transfer amount must be positive".to_string(),
            ));
        }

        let token = token_address.unwrap_or(self.address);
        let token_info = self.rpc.token_info().get_token_info(token).await?;

        let tx = match classify_direct_transfer_kind(&token_info.token_type) {
            Some(DirectTransferKind::Erc1155) => {
                let token_id = self
                    .core
                    .hub_v2()
                    .toTokenId(token)
                    .call()
                    .await
                    .map_err(|e| SdkError::Contract(e.to_string()))?;
                build_direct_erc1155_transfer_tx(
                    self.core.config.v2_hub_address,
                    self.address,
                    to,
                    token_id,
                    amount,
                    tx_data.unwrap_or_default(),
                )
            }
            Some(DirectTransferKind::Erc20) => build_direct_erc20_transfer_tx(token, to, amount),
            None => {
                return Err(SdkError::OperationFailed(format!(
                    "direct transfer is not supported for token type {}",
                    token_info.token_type
                )));
            }
        };

        Ok(vec![tx])
    }

    /// Execute a direct transfer using the runner (if present).
    pub async fn direct_transfer(
        &self,
        to: Address,
        amount: U256,
        token_address: Option<Address>,
        tx_data: Option<Bytes>,
    ) -> Result<Vec<crate::SubmittedTx>, SdkError> {
        let txs = self
            .plan_direct_transfer(to, amount, token_address, tx_data)
            .await?;
        self.send(txs).await
    }

    /// Plan an explicit ERC20 transfer without token-type lookup or pathfinding.
    pub async fn plan_transfer_erc20(
        &self,
        token: Address,
        to: Address,
        amount: U256,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        if amount.is_zero() {
            return Err(SdkError::OperationFailed(
                "ERC20 transfer amount must be positive".to_string(),
            ));
        }
        Ok(vec![build_direct_erc20_transfer_tx(token, to, amount)])
    }

    /// Execute an explicit ERC20 transfer using the runner.
    pub async fn transfer_erc20(
        &self,
        token: Address,
        to: Address,
        amount: U256,
    ) -> Result<Vec<crate::SubmittedTx>, SdkError> {
        let txs = self.plan_transfer_erc20(token, to, amount).await?;
        self.send(txs).await
    }

    /// Plan an explicit ERC1155 transfer through the Circles Hub.
    pub async fn plan_transfer_erc1155(
        &self,
        token: Address,
        to: Address,
        amount: U256,
        tx_data: Option<Bytes>,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        if amount.is_zero() {
            return Err(SdkError::OperationFailed(
                "ERC1155 transfer amount must be positive".to_string(),
            ));
        }
        Ok(vec![build_direct_erc1155_transfer_tx(
            self.core.config.v2_hub_address,
            self.address,
            to,
            address_to_erc1155_token_id(token),
            amount,
            tx_data.unwrap_or_default(),
        )])
    }

    /// Execute an explicit ERC1155 transfer through the Circles Hub using the runner.
    pub async fn transfer_erc1155(
        &self,
        token: Address,
        to: Address,
        amount: U256,
        tx_data: Option<Bytes>,
    ) -> Result<Vec<crate::SubmittedTx>, SdkError> {
        let txs = self
            .plan_transfer_erc1155(token, to, amount, tx_data)
            .await?;
        self.send(txs).await
    }

    /// Plan wrapping demurraged ERC20 Circles through HubV2::wrap.
    pub async fn plan_wrap_demurrage_erc20(
        &self,
        avatar: Address,
        amount: U256,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        Ok(vec![build_wrap_erc20_tx(
            self.core.config.v2_hub_address,
            avatar,
            amount,
            WrapperKind::Demurrage,
        )])
    }

    /// Plan wrapping inflationary ERC20 Circles through HubV2::wrap.
    pub async fn plan_wrap_inflation_erc20(
        &self,
        avatar: Address,
        amount: U256,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        Ok(vec![build_wrap_erc20_tx(
            self.core.config.v2_hub_address,
            avatar,
            amount,
            WrapperKind::Inflation,
        )])
    }

    /// Plan unwrapping demurraged ERC20 Circles through the wrapper token.
    pub async fn plan_unwrap_demurrage_erc20(
        &self,
        wrapper_token: Address,
        amount: U256,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        Ok(vec![build_unwrap_demurrage_erc20_tx(wrapper_token, amount)])
    }

    /// Plan unwrapping inflationary ERC20 Circles through the wrapper token.
    pub async fn plan_unwrap_inflation_erc20(
        &self,
        wrapper_token: Address,
        amount: U256,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        Ok(vec![build_unwrap_inflation_erc20_tx(wrapper_token, amount)])
    }

    /// Execute wrapping demurraged ERC20 Circles through the runner.
    pub async fn wrap_demurrage_erc20(
        &self,
        avatar: Address,
        amount: U256,
    ) -> Result<Vec<crate::SubmittedTx>, SdkError> {
        let txs = self.plan_wrap_demurrage_erc20(avatar, amount).await?;
        self.send(txs).await
    }

    /// Execute wrapping inflationary ERC20 Circles through the runner.
    pub async fn wrap_inflation_erc20(
        &self,
        avatar: Address,
        amount: U256,
    ) -> Result<Vec<crate::SubmittedTx>, SdkError> {
        let txs = self.plan_wrap_inflation_erc20(avatar, amount).await?;
        self.send(txs).await
    }

    /// Execute unwrapping demurraged ERC20 Circles through the runner.
    pub async fn unwrap_demurrage_erc20(
        &self,
        wrapper_token: Address,
        amount: U256,
    ) -> Result<Vec<crate::SubmittedTx>, SdkError> {
        let txs = self
            .plan_unwrap_demurrage_erc20(wrapper_token, amount)
            .await?;
        self.send(txs).await
    }

    /// Execute unwrapping inflationary ERC20 Circles through the runner.
    pub async fn unwrap_inflation_erc20(
        &self,
        wrapper_token: Address,
        amount: U256,
    ) -> Result<Vec<crate::SubmittedTx>, SdkError> {
        let txs = self
            .plan_unwrap_inflation_erc20(wrapper_token, amount)
            .await?;
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

    /// Plan a TS-style automatic group-token redeem flow for `group`.
    pub async fn plan_group_token_redeem(
        &self,
        group: Address,
        amount: U256,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        let builder = TransferBuilder::new(self.core.config.clone())?;
        let txs = builder
            .construct_group_token_redeem(self.address, group, amount)
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

    /// Execute a group-token redeem flow using the runner (if present).
    pub async fn group_token_redeem(
        &self,
        group: Address,
        amount: U256,
    ) -> Result<Vec<crate::SubmittedTx>, SdkError> {
        let txs = self.plan_group_token_redeem(group, amount).await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;
    use alloy_sol_types::SolCall;

    #[test]
    fn direct_transfer_classifies_ts_token_types() {
        assert_eq!(
            classify_direct_transfer_kind("CrcV2_RegisterHuman"),
            Some(DirectTransferKind::Erc1155)
        );
        assert_eq!(
            classify_direct_transfer_kind("CrcV2_RegisterGroup"),
            Some(DirectTransferKind::Erc1155)
        );
        assert_eq!(
            classify_direct_transfer_kind("CrcV2_ERC20WrapperDeployed_Demurraged"),
            Some(DirectTransferKind::Erc20)
        );
        assert_eq!(
            classify_direct_transfer_kind("CrcV2_ERC20WrapperDeployed_Inflationary"),
            Some(DirectTransferKind::Erc20)
        );
        assert_eq!(
            classify_direct_transfer_kind("CrcV2_RegisterOrganization"),
            None
        );
    }

    #[test]
    fn address_to_erc1155_token_id_matches_ts_uint256_conversion() {
        let token = address!("0000000000000000000000000000000000000007");
        assert_eq!(address_to_erc1155_token_id(token), U256::from(7u64));

        let token = address!("1234567890abcdef1234567890abcdef12345678");
        assert_eq!(
            address_to_erc1155_token_id(token),
            U256::from_be_slice(token.as_slice())
        );
    }

    #[test]
    fn erc1155_direct_transfer_tx_matches_hub_call() {
        let data = Bytes::from(vec![0xde, 0xad, 0xbe, 0xef]);
        let tx = build_direct_erc1155_transfer_tx(
            address!("1000000000000000000000000000000000000000"),
            address!("2000000000000000000000000000000000000000"),
            address!("3000000000000000000000000000000000000000"),
            U256::from(7u64),
            U256::from(9u64),
            data.clone(),
        );

        let expected = HubV2::safeTransferFromCall {
            _from: address!("2000000000000000000000000000000000000000"),
            _to: address!("3000000000000000000000000000000000000000"),
            _id: U256::from(7u64),
            _value: U256::from(9u64),
            _data: data,
        };

        assert_eq!(tx.to, address!("1000000000000000000000000000000000000000"));
        assert_eq!(tx.data, Bytes::from(expected.abi_encode()));
        assert_eq!(tx.value, None);
    }

    #[test]
    fn erc20_direct_transfer_tx_matches_transfer_call() {
        let tx = build_direct_erc20_transfer_tx(
            address!("4000000000000000000000000000000000000000"),
            address!("5000000000000000000000000000000000000000"),
            U256::from(42u64),
        );

        let expected = IERC20Like::transferCall {
            to: address!("5000000000000000000000000000000000000000"),
            value: U256::from(42u64),
        };

        assert_eq!(tx.to, address!("4000000000000000000000000000000000000000"));
        assert_eq!(tx.data, Bytes::from(expected.abi_encode()));
        assert_eq!(tx.value, None);
    }

    #[test]
    fn wrap_demurrage_erc20_plan_matches_hub_wrap_call() {
        let hub = address!("1000000000000000000000000000000000000000");
        let avatar = address!("2000000000000000000000000000000000000000");
        let tx = build_wrap_erc20_tx(hub, avatar, U256::from(42u64), WrapperKind::Demurrage);

        let expected = HubV2::wrapCall {
            _avatar: avatar,
            _amount: U256::from(42u64),
            _type: 0,
        };

        assert_eq!(tx.to, hub);
        assert_eq!(tx.data, Bytes::from(expected.abi_encode()));
        assert_eq!(tx.value, None);
    }

    #[test]
    fn wrap_inflation_erc20_plan_matches_hub_wrap_call() {
        let hub = address!("1000000000000000000000000000000000000000");
        let avatar = address!("2000000000000000000000000000000000000000");
        let tx = build_wrap_erc20_tx(hub, avatar, U256::from(42u64), WrapperKind::Inflation);

        let expected = HubV2::wrapCall {
            _avatar: avatar,
            _amount: U256::from(42u64),
            _type: 1,
        };

        assert_eq!(tx.to, hub);
        assert_eq!(tx.data, Bytes::from(expected.abi_encode()));
        assert_eq!(tx.value, None);
    }

    #[test]
    fn unwrap_demurrage_erc20_plan_matches_wrapper_unwrap_call() {
        let wrapper = address!("3000000000000000000000000000000000000000");
        let tx = build_unwrap_demurrage_erc20_tx(wrapper, U256::from(42u64));
        let expected = DemurrageCircles::unwrapCall {
            _amount: U256::from(42u64),
        };

        assert_eq!(tx.to, wrapper);
        assert_eq!(tx.data, Bytes::from(expected.abi_encode()));
        assert_eq!(tx.value, None);
    }

    #[test]
    fn unwrap_inflation_erc20_plan_matches_wrapper_unwrap_call() {
        let wrapper = address!("3000000000000000000000000000000000000000");
        let tx = build_unwrap_inflation_erc20_tx(wrapper, U256::from(42u64));
        let expected = InflationaryCircles::unwrapCall {
            _amount: U256::from(42u64),
        };

        assert_eq!(tx.to, wrapper);
        assert_eq!(tx.data, Bytes::from(expected.abi_encode()));
        assert_eq!(tx.value, None);
    }
}
