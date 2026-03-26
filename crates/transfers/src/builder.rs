use crate::error::{TransferError, TransfersErrorSource};
use alloy_primitives::{aliases::U96, Address, Bytes, U256};
use alloy_sol_types::SolCall;
use circles_abis::{DemurrageCircles, HubV2, InflationaryCircles, LiftERC20};
use circles_pathfinder::{
    create_flow_matrix, expected_unwrapped_totals, replace_wrapped_tokens,
    token_info_map_from_path, wrapped_totals_from_path,
};
use circles_rpc::CirclesRpc;
use circles_types::{
    AdvancedTransferOptions, Balance, CirclesConfig, FindPathParams, PathfindingTransferStep,
    SimulatedTrust, TokenBalanceResponse, TokenInfo, TransferStep,
};
use circles_utils::converter::{
    atto_circles_to_atto_static_circles, atto_static_circles_to_atto_circles,
};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

/// Simple transfer transaction representation.
#[derive(Debug, Clone)]
pub struct TransferTx {
    pub to: Address,
    pub data: Bytes,
    pub value: U256,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ReplenishBalances {
    unwrapped_balance: U256,
    wrapped_demurrage_balance: U256,
    wrapped_inflationary_balance: U256,
    wrapped_demurrage_address: Option<Address>,
    wrapped_inflationary_address: Option<Address>,
}

impl ReplenishBalances {
    fn total_available(&self) -> U256 {
        self.unwrapped_balance
            + self.wrapped_demurrage_balance
            + atto_static_circles_to_atto_circles(self.wrapped_inflationary_balance, None)
    }
}

/// High-level builder for Circles transfers.
///
/// Mirrors the TS `TransferBuilder`: finds a path, handles wrappers, and
/// returns the ordered transaction list (approval, unwraps, operateFlowMatrix,
/// re-wraps TBD).
pub struct TransferBuilder {
    config: CirclesConfig,
    rpc: CirclesRpc,
    /// If false, will skip the approval check and always include approval.
    check_approval: bool,
}

impl TransferBuilder {
    /// Create a new builder from a Circles config.
    ///
    /// Uses the config's `circles_rpc_url` for pathfinding + balances; does not
    /// submit transactions (pair with a runner in the SDK to send).
    pub fn new(config: CirclesConfig) -> Result<Self, TransferError> {
        let rpc = CirclesRpc::try_from(config.circles_rpc_url.as_str()).map_err(|e| {
            TransferError::generic(
                e.to_string(),
                None::<String>,
                TransfersErrorSource::Transfers,
            )
        })?;
        Ok(Self {
            config,
            rpc,
            check_approval: true,
        })
    }

    pub fn config(&self) -> &CirclesConfig {
        &self.config
    }

    /// Control whether approval is checked (default: true).
    ///
    /// If `false`, the safety approval (`setApprovalForAll`) is always included
    /// without checking existing approval status.
    pub fn with_approval_check(mut self, check: bool) -> Self {
        self.check_approval = check;
        self
    }

    /// Construct an advanced transfer and return the ordered transaction list.
    ///
    /// Flow: optional self-unwrap fast-path (from==to, single token pair),
    /// pathfind with wrapped balances by default, unwrap inflationary/demurraged
    /// wrappers, operateFlowMatrix, and re-wrap inflationary leftovers when
    /// static balances are available. Does not send transactions.
    pub async fn construct_advanced_transfer(
        &self,
        from: Address,
        to: Address,
        amount: U256,
        options: Option<AdvancedTransferOptions>,
    ) -> Result<Vec<TransferTx>, TransferError> {
        self.construct_advanced_transfer_with_aggregate(from, to, amount, options, false)
            .await
    }

    /// Construct an advanced transfer and optionally append the TS-style
    /// recipient aggregation self-transfer when a single `to_token` is selected.
    pub async fn construct_advanced_transfer_with_aggregate(
        &self,
        from: Address,
        to: Address,
        amount: U256,
        options: Option<AdvancedTransferOptions>,
        aggregate: bool,
    ) -> Result<Vec<TransferTx>, TransferError> {
        // Self-transfer fast-path for unwrap: if from == to and from/to tokens are provided and distinct.
        if from == to {
            if let Some(ref opts) = options {
                if let (Some(from_tokens), Some(to_tokens)) =
                    (opts.from_tokens.as_ref(), opts.to_tokens.as_ref())
                {
                    if from_tokens.len() == 1 && to_tokens.len() == 1 {
                        let from_token = from_tokens[0];
                        let to_token = to_tokens[0];
                        if from_token != to_token {
                            // Attempt unwrap via wrapper contracts
                            if let Some(tx) = self.self_unwrap(from_token, to_token, amount).await?
                            {
                                return Ok(vec![tx]);
                            }
                        }
                    }
                }
            }
        }

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

        let target_flow = truncate_to_six_decimals(amount);

        // Pathfinding
        let opts_for_path = opts.clone();
        let params = FindPathParams {
            from,
            to,
            target_flow,
            use_wrapped_balances: opts_for_path.use_wrapped_balances,
            from_tokens: opts_for_path.from_tokens,
            to_tokens: opts_for_path.to_tokens,
            exclude_from_tokens: opts_for_path.exclude_from_tokens,
            exclude_to_tokens: opts_for_path.exclude_to_tokens,
            simulated_balances: opts_for_path.simulated_balances,
            simulated_trusts: opts_for_path.simulated_trusts,
            max_transfers: opts_for_path.max_transfers,
        };

        let path = self.rpc.pathfinder().find_path(params).await.map_err(|e| {
            TransferError::generic(
                e.to_string(),
                None::<String>,
                TransfersErrorSource::Pathfinding,
            )
        })?;

        if path.transfers.is_empty() {
            return Err(TransferError::no_path_found(from, to, None));
        }
        if path.max_flow < target_flow {
            return Err(TransferError::insufficient_balance(
                target_flow,
                path.max_flow,
                from,
                to,
            ));
        }

        let path = maybe_add_aggregate_transfer(path, to, &opts, aggregate);

        // Token info + wrapper bookkeeping
        let token_info_map = token_info_map_from_path(from, &self.rpc, &path)
            .await
            .map_err(|e| {
                TransferError::generic(
                    e.to_string(),
                    None::<String>,
                    TransfersErrorSource::Pathfinding,
                )
            })?;
        let wrapped_totals = wrapped_totals_from_path(&path, &token_info_map);
        let has_wrapped = !wrapped_totals.is_empty();

        validate_wrapped_balance_usage(has_wrapped, opts.use_wrapped_balances)?;

        // Fetch balances once (for inflationary leftover wrap).
        let balance_map = if has_wrapped {
            self.fetch_static_balances(from).await?
        } else {
            HashMap::new()
        };

        self.assemble_transactions_inner(
            from,
            to,
            path,
            token_info_map,
            wrapped_totals,
            balance_map,
            opts,
            true,
        )
    }

    /// Construct the TS-style replenish flow: use existing unwrapped balance
    /// first, then unwrap local wrappers, and only pathfind the remaining
    /// deficit when necessary.
    pub async fn construct_replenish(
        &self,
        from: Address,
        token_id: Address,
        amount: U256,
        receiver: Option<Address>,
    ) -> Result<Vec<TransferTx>, TransferError> {
        let receiver = receiver.unwrap_or(from);
        let balances = self.fetch_replenish_balances(from, token_id).await?;
        let total_available = balances.total_available();
        let mut transactions = Vec::new();

        if balances.unwrapped_balance >= amount {
            if receiver != from {
                transactions.push(
                    self.safe_transfer_tx(from, receiver, token_id, amount)
                        .await?,
                );
            }
            return Ok(transactions);
        }

        let deficit = amount.saturating_sub(balances.unwrapped_balance);

        if total_available >= amount {
            transactions.extend(create_replenish_unwraps(&balances, deficit));
            if receiver != from {
                transactions.push(
                    self.safe_transfer_tx(from, receiver, token_id, amount)
                        .await?,
                );
            }
            return Ok(transactions);
        }

        let needs_temporary_trust = !self.is_trusted(from, token_id).await?;
        let simulated_trusts = needs_temporary_trust.then_some(vec![SimulatedTrust {
            truster: from,
            trustee: token_id,
        }]);
        let rounded_up_deficit = round_up_to_six_decimals(deficit);

        let path = self
            .rpc
            .pathfinder()
            .find_path(FindPathParams {
                from,
                to: receiver,
                target_flow: rounded_up_deficit,
                use_wrapped_balances: Some(true),
                from_tokens: None,
                to_tokens: Some(vec![token_id]),
                exclude_from_tokens: None,
                exclude_to_tokens: None,
                simulated_balances: None,
                simulated_trusts: simulated_trusts.clone(),
                max_transfers: None,
            })
            .await
            .map_err(|e| {
                replenish_pathfinding_error(
                    e.to_string(),
                    amount,
                    &balances,
                    total_available,
                    from,
                    token_id,
                )
            })?;

        if path.transfers.is_empty() {
            return Err(TransferError::no_path_found(
                from,
                receiver,
                Some(format!("No path to acquire token {token_id:#x}")),
            ));
        }
        if path.max_flow < rounded_up_deficit {
            return Err(TransferError::generic(
                format!(
                    "Pathfinder can only provide {} wei of the {} wei deficit needed for token {token_id:#x}.",
                    path.max_flow, rounded_up_deficit
                ),
                Some("REPLENISH_INSUFFICIENT_PATH_FLOW"),
                TransfersErrorSource::Pathfinding,
            ));
        }

        if needs_temporary_trust {
            transactions.push(self.trust_tx(token_id, replenish_trust_expiry()));
        }

        let token_info_map = token_info_map_from_path(from, &self.rpc, &path)
            .await
            .map_err(|e| {
                TransferError::generic(
                    e.to_string(),
                    None::<String>,
                    TransfersErrorSource::Pathfinding,
                )
            })?;
        let wrapped_totals = wrapped_totals_from_path(&path, &token_info_map);
        let balance_map = if wrapped_totals.is_empty() {
            HashMap::new()
        } else {
            self.fetch_static_balances(from).await?
        };
        let opts = AdvancedTransferOptions {
            use_wrapped_balances: Some(true),
            from_tokens: None,
            to_tokens: Some(vec![token_id]),
            exclude_from_tokens: None,
            exclude_to_tokens: None,
            simulated_balances: None,
            simulated_trusts,
            max_transfers: None,
            tx_data: None,
        };

        let mut replenish_txs = self.assemble_transactions_inner(
            from,
            receiver,
            path,
            token_info_map,
            wrapped_totals,
            balance_map,
            opts,
            true,
        )?;
        transactions.append(&mut replenish_txs);

        if needs_temporary_trust {
            transactions.push(self.trust_tx(token_id, U96::ZERO));
        }

        Ok(transactions)
    }

    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    pub fn assemble_transactions_inner(
        &self,
        from: Address,
        to: Address,
        path: circles_types::PathfindingResult,
        token_info_map: HashMap<Address, circles_types::TokenInfo>,
        wrapped_totals: HashMap<Address, (U256, String)>,
        balance_map: HashMap<Address, U256>,
        opts: AdvancedTransferOptions,
        check_approval: bool,
    ) -> Result<Vec<TransferTx>, TransferError> {
        let unwrapped_map = expected_unwrapped_totals(&wrapped_totals, &token_info_map);

        // Build unwrap calls
        let mut unwraps = Vec::new();
        let mut rewraps = Vec::new();
        for (wrapper, (amount_dem, _owner)) in &unwrapped_map {
            if let Some(info) = token_info_map.get(wrapper) {
                if info.token_type == "CrcV2_ERC20WrapperDeployed_Demurraged" {
                    let call = DemurrageCircles::unwrapCall {
                        _amount: *amount_dem,
                    };
                    unwraps.push(TransferTx {
                        to: *wrapper,
                        data: Bytes::from(call.abi_encode()),
                        value: U256::ZERO,
                    });
                } else if info.token_type == "CrcV2_ERC20WrapperDeployed_Inflationary" {
                    // Unwrap only the amount used in the path, converted with
                    // current-time semantics to match the TS TransferBuilder.
                    let static_amt = atto_circles_to_atto_static_circles(*amount_dem, None);
                    let call = InflationaryCircles::unwrapCall {
                        _amount: static_amt,
                    };
                    unwraps.push(TransferTx {
                        to: *wrapper,
                        data: Bytes::from(call.abi_encode()),
                        value: U256::ZERO,
                    });

                    // Re-wrap leftover if any (current static balance - used static)
                    if let Some(current_static) = balance_map.get(wrapper) {
                        let leftover = current_static.saturating_sub(static_amt);
                        if leftover > U256::ZERO {
                            let owner = info.token_owner;
                            let wrap_call = HubV2::wrapCall {
                                _avatar: owner,
                                _amount: leftover,
                                _type: 1u8, // Inflationary
                            };
                            rewraps.push(TransferTx {
                                to: self.config.v2_hub_address,
                                data: Bytes::from(wrap_call.abi_encode()),
                                value: U256::ZERO,
                            });
                        }
                    }
                }
            }
        }

        // Replace wrapped tokens in path for flow matrix
        let unwrapped_addr_map: HashMap<Address, (U256, Address)> = unwrapped_map.clone();
        let path_unwrapped = replace_wrapped_tokens(&path, &unwrapped_addr_map);

        // Build TransferStep list for flow matrix
        let transfers = path_unwrapped
            .transfers
            .iter()
            .filter_map(|t| {
                let token_owner = Address::from_str(&t.token_owner).ok()?;
                Some(TransferStep {
                    from_address: t.from,
                    to_address: t.to,
                    token_owner,
                    value: u256_to_u192_local(t.value),
                })
            })
            .collect::<Vec<_>>();

        let mut flow_matrix =
            create_flow_matrix(from, to, u256_to_u192_local(path.max_flow), &transfers).map_err(
                |e| {
                    TransferError::generic(
                        e.to_string(),
                        None::<String>,
                        TransfersErrorSource::FlowMatrix,
                    )
                },
            )?;

        if let Some(tx_data) = opts.tx_data {
            if let Some(first) = flow_matrix.streams.get_mut(0) {
                first.data = tx_data;
            }
        }

        // operateFlowMatrix
        let flow_edges: Vec<_> = flow_matrix
            .flow_edges
            .iter()
            .map(|e| (e.streamSinkId, e.amount))
            .collect();
        let streams: Vec<_> = flow_matrix
            .streams
            .iter()
            .map(|s| (s.sourceCoordinate, s.flowEdgeIds.clone(), s.data.clone()))
            .collect();

        let op_call = HubV2::operateFlowMatrixCall {
            _flowVertices: flow_matrix.flow_vertices.clone(),
            _flow: flow_edges,
            _streams: streams,
            _packedCoordinates: Bytes::from(flow_matrix.packed_coordinates.clone()),
        };

        let mut txs = Vec::new();
        // In test/fixture contexts we skip the approval check and always include it to avoid async DNS.
        let needs_approval = if check_approval && self.check_approval {
            needs_approval_blocking(|| self.needs_approval(from)).unwrap_or(true)
        } else {
            true
        };
        if needs_approval {
            let approve_call = HubV2::setApprovalForAllCall {
                _operator: from,
                _approved: true,
            };
            txs.push(TransferTx {
                to: self.config.v2_hub_address,
                data: Bytes::from(approve_call.abi_encode()),
                value: U256::ZERO,
            });
        }
        txs.extend(unwraps);
        txs.push(TransferTx {
            to: self.config.v2_hub_address,
            data: Bytes::from(op_call.abi_encode()),
            value: U256::ZERO,
        });

        txs.extend(rewraps);

        Ok(txs)
    }

    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    pub fn assemble_transactions(
        &self,
        from: Address,
        to: Address,
        path: circles_types::PathfindingResult,
        token_info_map: HashMap<Address, circles_types::TokenInfo>,
        wrapped_totals: HashMap<Address, (U256, String)>,
        balance_map: HashMap<Address, U256>,
        opts: AdvancedTransferOptions,
        check_approval: bool,
    ) -> Result<Vec<TransferTx>, TransferError> {
        self.assemble_transactions_inner(
            from,
            to,
            path,
            token_info_map,
            wrapped_totals,
            balance_map,
            opts,
            check_approval,
        )
    }

    async fn fetch_static_balances(
        &self,
        avatar: Address,
    ) -> Result<HashMap<Address, U256>, TransferError> {
        let balances: Vec<TokenBalanceResponse> = self
            .rpc
            .token()
            .get_token_balances(avatar, false, true)
            .await
            .map_err(|e| {
                TransferError::generic(
                    e.to_string(),
                    None::<String>,
                    TransfersErrorSource::Transfers,
                )
            })?;

        let mut map = HashMap::new();
        for b in balances {
            if let Some(static_amt) = b.static_atto_circles {
                map.insert(b.token_id, static_amt);
            }
        }
        Ok(map)
    }

    async fn fetch_replenish_balances(
        &self,
        avatar: Address,
        token_id: Address,
    ) -> Result<ReplenishBalances, TransferError> {
        let balances: Vec<TokenBalanceResponse> = self
            .rpc
            .token()
            .get_token_balances(avatar, false, true)
            .await
            .map_err(|e| {
                TransferError::generic(
                    e.to_string(),
                    None::<String>,
                    TransfersErrorSource::Transfers,
                )
            })?;

        let relevant_balances = balances
            .into_iter()
            .filter(|balance| balance.token_owner == token_id)
            .collect::<Vec<_>>();
        if relevant_balances.is_empty() {
            return Ok(ReplenishBalances::default());
        }

        let token_ids = relevant_balances
            .iter()
            .map(|balance| balance.token_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let token_info_map = self
            .rpc
            .token_info()
            .get_token_info_batch(token_ids)
            .await
            .map_err(|e| {
                TransferError::generic(
                    e.to_string(),
                    None::<String>,
                    TransfersErrorSource::Transfers,
                )
            })?
            .into_iter()
            .map(|info| (info.token, info))
            .collect::<HashMap<_, _>>();

        classify_replenish_balances(relevant_balances, &token_info_map)
    }
}

fn truncate_to_six_decimals(amount: U256) -> U256 {
    let unit = U256::from(1_000_000_000_000u64); // 1e12 wei granularity
    (amount / unit) * unit
}

fn round_up_to_six_decimals(amount: U256) -> U256 {
    let truncated = truncate_to_six_decimals(amount);
    if truncated == amount {
        amount
    } else {
        truncated + U256::from(1_000_000_000_000u64)
    }
}

fn u256_to_u192_local(value: U256) -> alloy_primitives::aliases::U192 {
    use alloy_primitives::aliases::U192;
    let limbs = value.into_limbs();
    if limbs[3] != 0 {
        U192::MAX
    } else {
        U192::from_limbs([limbs[0], limbs[1], limbs[2]])
    }
}

fn validate_wrapped_balance_usage(
    has_wrapped: bool,
    use_wrapped_balances: Option<bool>,
) -> Result<(), TransferError> {
    if has_wrapped && !use_wrapped_balances.unwrap_or(false) {
        return Err(TransferError::wrapped_tokens_required());
    }
    Ok(())
}

fn maybe_add_aggregate_transfer(
    mut path: circles_types::PathfindingResult,
    to: Address,
    opts: &AdvancedTransferOptions,
    aggregate: bool,
) -> circles_types::PathfindingResult {
    let Some(to_tokens) = opts.to_tokens.as_ref() else {
        return path;
    };

    if aggregate && to_tokens.len() == 1 && path.max_flow > U256::ZERO {
        path.transfers.push(PathfindingTransferStep {
            from: to,
            to,
            token_owner: format!("{:#x}", to_tokens[0]),
            value: path.max_flow,
        });
    }

    path
}

fn create_replenish_unwraps(balances: &ReplenishBalances, deficit: U256) -> Vec<TransferTx> {
    let mut remaining_to_unwrap = deficit;
    let mut txs = Vec::new();

    if remaining_to_unwrap > U256::ZERO && balances.wrapped_demurrage_balance > U256::ZERO {
        if let Some(wrapper) = balances.wrapped_demurrage_address {
            let to_unwrap = remaining_to_unwrap.min(balances.wrapped_demurrage_balance);
            let call = DemurrageCircles::unwrapCall { _amount: to_unwrap };
            txs.push(TransferTx {
                to: wrapper,
                data: Bytes::from(call.abi_encode()),
                value: U256::ZERO,
            });
            remaining_to_unwrap = remaining_to_unwrap.saturating_sub(to_unwrap);
        }
    }

    if remaining_to_unwrap > U256::ZERO && balances.wrapped_inflationary_balance > U256::ZERO {
        if let Some(wrapper) = balances.wrapped_inflationary_address {
            let static_to_unwrap = atto_circles_to_atto_static_circles(remaining_to_unwrap, None);
            let actual_unwrap = static_to_unwrap.min(balances.wrapped_inflationary_balance);
            let call = InflationaryCircles::unwrapCall {
                _amount: actual_unwrap,
            };
            txs.push(TransferTx {
                to: wrapper,
                data: Bytes::from(call.abi_encode()),
                value: U256::ZERO,
            });
        }
    }

    txs
}

fn extract_raw_balance(balance: &Balance) -> Result<U256, TransferError> {
    match balance {
        Balance::Raw(value) => Ok(*value),
        Balance::TimeCircles(_) => Err(TransferError::generic(
            "Expected raw token balance from circlesV2_getTokenBalances, got timeCircles output.",
            Some("UNEXPECTED_TIME_CIRCLES_BALANCE"),
            TransfersErrorSource::Validation,
        )),
    }
}

fn classify_replenish_balances(
    balances: Vec<TokenBalanceResponse>,
    token_info_map: &HashMap<Address, TokenInfo>,
) -> Result<ReplenishBalances, TransferError> {
    let mut replenishment = ReplenishBalances::default();

    for balance in balances {
        let token_info = token_info_map.get(&balance.token_id).ok_or_else(|| {
            TransferError::generic(
                format!(
                    "Missing token metadata for replenish balance token {:#x}.",
                    balance.token_id
                ),
                Some("REPLENISH_MISSING_TOKEN_INFO"),
                TransfersErrorSource::Transfers,
            )
        })?;
        let raw_balance = extract_raw_balance(&balance.balance)?;

        match token_info.token_type.as_str() {
            "CrcV2_ERC20WrapperDeployed_Demurraged" => {
                replenishment.wrapped_demurrage_balance = raw_balance;
                replenishment.wrapped_demurrage_address = Some(balance.token_id);
            }
            "CrcV2_ERC20WrapperDeployed_Inflationary" => {
                replenishment.wrapped_inflationary_balance =
                    balance.static_atto_circles.unwrap_or_default();
                replenishment.wrapped_inflationary_address = Some(balance.token_id);
            }
            _ => {
                replenishment.unwrapped_balance = raw_balance;
            }
        }
    }

    Ok(replenishment)
}

fn replenish_pathfinding_error(
    message: String,
    amount: U256,
    balances: &ReplenishBalances,
    total_available: U256,
    from: Address,
    token_id: Address,
) -> TransferError {
    let deficit = amount.saturating_sub(balances.unwrapped_balance);
    let unreachable =
        deficit.saturating_sub(total_available.saturating_sub(balances.unwrapped_balance));

    TransferError::generic(
        format!(
            "Insufficient tokens to replenish from {from:#x} for token {token_id:#x}. Target: {amount} wei, current unwrapped: {} wei, need: {deficit} wei, available: {total_available} wei. Cannot acquire the remaining {unreachable} wei. RPC/pathfinder error: {message}",
            balances.unwrapped_balance,
        ),
        Some("REPLENISH_INSUFFICIENT_TOKENS"),
        TransfersErrorSource::Validation,
    )
}

fn replenish_trust_expiry() -> U96 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let expiry = now + (365 * 24 * 60 * 60);
    U96::from(expiry)
}

impl TransferBuilder {
    async fn needs_approval(&self, operator: Address) -> Option<bool> {
        let Ok(url) = self.config.circles_rpc_url.parse() else {
            return None;
        };
        let provider = alloy_provider::ProviderBuilder::new().connect_http(url);
        let hub = HubV2::new(self.config.v2_hub_address, provider);
        match hub.isApprovedForAll(operator, operator).call().await {
            Ok(resp) => Some(!resp),
            Err(_) => None,
        }
    }

    async fn self_unwrap(
        &self,
        from_token: Address,
        to_token: Address,
        amount: U256,
    ) -> Result<Option<TransferTx>, TransferError> {
        // Resolve wrapper type via LiftERC20
        let Ok(url) = self.config.circles_rpc_url.parse() else {
            return Ok(None);
        };
        let provider = alloy_provider::ProviderBuilder::new().connect_http(url);
        let lift = LiftERC20::new(self.config.lift_erc20_address, provider);
        let dem = lift
            .erc20Circles(0u8, to_token)
            .call()
            .await
            .map_err(|e| {
                TransferError::generic(
                    e.to_string(),
                    None::<String>,
                    TransfersErrorSource::Transfers,
                )
            })?
            .0;
        let inf = lift
            .erc20Circles(1u8, to_token)
            .call()
            .await
            .map_err(|e| {
                TransferError::generic(
                    e.to_string(),
                    None::<String>,
                    TransfersErrorSource::Transfers,
                )
            })?
            .0;

        let dem_addr = Address::from(dem);
        let inf_addr = Address::from(inf);

        if from_token == dem_addr {
            let demurrage_call = DemurrageCircles::unwrapCall { _amount: amount };
            return Ok(Some(TransferTx {
                to: from_token,
                data: Bytes::from(demurrage_call.abi_encode()),
                value: U256::ZERO,
            }));
        } else if from_token == inf_addr {
            let static_amt = atto_circles_to_atto_static_circles(amount, None);
            let infl_call = InflationaryCircles::unwrapCall {
                _amount: static_amt,
            };
            return Ok(Some(TransferTx {
                to: from_token,
                data: Bytes::from(infl_call.abi_encode()),
                value: U256::ZERO,
            }));
        }
        Ok(None)
    }

    async fn is_trusted(&self, truster: Address, trustee: Address) -> Result<bool, TransferError> {
        let Ok(url) = self.config.circles_rpc_url.parse() else {
            return Err(TransferError::generic(
                "invalid circles rpc url",
                None::<String>,
                TransfersErrorSource::Transfers,
            ));
        };
        let provider = alloy_provider::ProviderBuilder::new().connect_http(url);
        let hub = HubV2::new(self.config.v2_hub_address, provider);
        hub.isTrusted(truster, trustee).call().await.map_err(|e| {
            TransferError::generic(
                e.to_string(),
                None::<String>,
                TransfersErrorSource::Transfers,
            )
        })
    }

    async fn safe_transfer_tx(
        &self,
        from: Address,
        to: Address,
        token_id: Address,
        amount: U256,
    ) -> Result<TransferTx, TransferError> {
        let Ok(url) = self.config.circles_rpc_url.parse() else {
            return Err(TransferError::generic(
                "invalid circles rpc url",
                None::<String>,
                TransfersErrorSource::Transfers,
            ));
        };
        let provider = alloy_provider::ProviderBuilder::new().connect_http(url);
        let hub = HubV2::new(self.config.v2_hub_address, provider);
        let erc1155_id = hub.toTokenId(token_id).call().await.map_err(|e| {
            TransferError::generic(
                e.to_string(),
                None::<String>,
                TransfersErrorSource::Transfers,
            )
        })?;
        let call = HubV2::safeTransferFromCall {
            _from: from,
            _to: to,
            _id: erc1155_id,
            _value: amount,
            _data: Bytes::default(),
        };
        Ok(TransferTx {
            to: self.config.v2_hub_address,
            data: Bytes::from(call.abi_encode()),
            value: U256::ZERO,
        })
    }

    fn trust_tx(&self, trust_receiver: Address, expiry: U96) -> TransferTx {
        let call = HubV2::trustCall {
            _trustReceiver: trust_receiver,
            _expiry: expiry,
        };
        TransferTx {
            to: self.config.v2_hub_address,
            data: Bytes::from(call.abi_encode()),
            value: U256::ZERO,
        }
    }
}

fn needs_approval_blocking<F, Fut>(f: F) -> Option<bool>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Option<bool>>,
{
    futures::executor::block_on(f())
}

#[cfg(test)]
mod tests {
    use super::{
        classify_replenish_balances, create_replenish_unwraps, maybe_add_aggregate_transfer,
        round_up_to_six_decimals, validate_wrapped_balance_usage, ReplenishBalances,
    };
    use crate::TransferError;
    use alloy_primitives::{address, fixed_bytes, Address, TxHash, U256};
    use circles_types::{
        AdvancedTransferOptions, Balance, PathfindingResult, PathfindingTransferStep,
        TokenBalanceResponse, TokenInfo,
    };
    use std::collections::HashMap;

    #[test]
    fn wrapped_balance_guard_matches_ts_behavior() {
        assert!(validate_wrapped_balance_usage(true, Some(true)).is_ok());
        assert!(validate_wrapped_balance_usage(false, Some(false)).is_ok());
        assert!(matches!(
            validate_wrapped_balance_usage(true, Some(false)),
            Err(TransferError::WrappedTokensRequired)
        ));
        assert!(matches!(
            validate_wrapped_balance_usage(true, None),
            Err(TransferError::WrappedTokensRequired)
        ));
    }

    #[test]
    fn aggregate_transfer_is_appended_for_single_to_token() {
        let source = address!("0x1000000000000000000000000000000000000001");
        let sink = address!("0x2000000000000000000000000000000000000002");
        let aggregate_token = address!("0x3000000000000000000000000000000000000003");
        let path = PathfindingResult {
            max_flow: U256::from(5u64),
            transfers: vec![PathfindingTransferStep {
                from: source,
                to: sink,
                token_owner: format!("{source:#x}"),
                value: U256::from(5u64),
            }],
        };
        let opts = AdvancedTransferOptions {
            use_wrapped_balances: Some(true),
            from_tokens: None,
            to_tokens: Some(vec![aggregate_token]),
            exclude_from_tokens: None,
            exclude_to_tokens: None,
            simulated_balances: None,
            simulated_trusts: None,
            max_transfers: None,
            tx_data: None,
        };

        let aggregated = maybe_add_aggregate_transfer(path, sink, &opts, true);
        let appended = aggregated.transfers.last().unwrap();

        assert_eq!(aggregated.transfers.len(), 2);
        assert_eq!(appended.from, sink);
        assert_eq!(appended.to, sink);
        assert_eq!(appended.token_owner, format!("{aggregate_token:#x}"));
        assert_eq!(appended.value, U256::from(5u64));
    }

    #[test]
    fn aggregate_transfer_is_not_appended_without_single_to_token() {
        let source = address!("0x4000000000000000000000000000000000000004");
        let sink = address!("0x5000000000000000000000000000000000000005");
        let path = PathfindingResult {
            max_flow: U256::from(5u64),
            transfers: vec![PathfindingTransferStep {
                from: source,
                to: sink,
                token_owner: format!("{source:#x}"),
                value: U256::from(5u64),
            }],
        };
        let opts = AdvancedTransferOptions {
            use_wrapped_balances: Some(true),
            from_tokens: None,
            to_tokens: Some(vec![
                address!("0x6000000000000000000000000000000000000006"),
                address!("0x7000000000000000000000000000000000000007"),
            ]),
            exclude_from_tokens: None,
            exclude_to_tokens: None,
            simulated_balances: None,
            simulated_trusts: None,
            max_transfers: None,
            tx_data: None,
        };

        let unchanged = maybe_add_aggregate_transfer(path, sink, &opts, true);

        assert_eq!(unchanged.transfers.len(), 1);
    }

    #[test]
    fn round_up_to_six_decimals_matches_ts_replenish_behavior() {
        assert_eq!(round_up_to_six_decimals(U256::from(0u64)), U256::ZERO);
        assert_eq!(
            round_up_to_six_decimals(U256::from(1_000_000_000_000u64)),
            U256::from(1_000_000_000_000u64)
        );
        assert_eq!(
            round_up_to_six_decimals(U256::from(1_000_000_000_001u64)),
            U256::from(2_000_000_000_000u64)
        );
    }

    #[test]
    fn classify_replenish_balances_matches_ts_bucketing() {
        let token_owner = address!("1111111111111111111111111111111111111111");
        let dem_wrapper = address!("2222222222222222222222222222222222222222");
        let inf_wrapper = address!("3333333333333333333333333333333333333333");
        let mut token_info_map = HashMap::new();
        token_info_map.insert(
            token_owner,
            mock_token_info(token_owner, token_owner, "CrcV2_RegisterHuman"),
        );
        token_info_map.insert(
            dem_wrapper,
            mock_token_info(
                dem_wrapper,
                token_owner,
                "CrcV2_ERC20WrapperDeployed_Demurraged",
            ),
        );
        token_info_map.insert(
            inf_wrapper,
            mock_token_info(
                inf_wrapper,
                token_owner,
                "CrcV2_ERC20WrapperDeployed_Inflationary",
            ),
        );

        let balances = vec![
            TokenBalanceResponse {
                token_id: token_owner,
                balance: Balance::Raw(U256::from(10u64)),
                static_atto_circles: None,
                static_circles: None,
                token_owner,
            },
            TokenBalanceResponse {
                token_id: dem_wrapper,
                balance: Balance::Raw(U256::from(20u64)),
                static_atto_circles: Some(U256::from(20u64)),
                static_circles: None,
                token_owner,
            },
            TokenBalanceResponse {
                token_id: inf_wrapper,
                balance: Balance::Raw(U256::from(30u64)),
                static_atto_circles: Some(U256::from(40u64)),
                static_circles: None,
                token_owner,
            },
        ];

        let classified = classify_replenish_balances(balances, &token_info_map).unwrap();

        assert_eq!(classified.unwrapped_balance, U256::from(10u64));
        assert_eq!(classified.wrapped_demurrage_balance, U256::from(20u64));
        assert_eq!(classified.wrapped_demurrage_address, Some(dem_wrapper));
        assert_eq!(classified.wrapped_inflationary_balance, U256::from(40u64));
        assert_eq!(classified.wrapped_inflationary_address, Some(inf_wrapper));
    }

    #[test]
    fn create_replenish_unwraps_prefers_demurrage_then_inflationary() {
        let dem_wrapper = address!("4444444444444444444444444444444444444444");
        let inf_wrapper = address!("5555555555555555555555555555555555555555");
        let balances = ReplenishBalances {
            unwrapped_balance: U256::ZERO,
            wrapped_demurrage_balance: U256::from(7u64),
            wrapped_inflationary_balance: U256::from(9u64),
            wrapped_demurrage_address: Some(dem_wrapper),
            wrapped_inflationary_address: Some(inf_wrapper),
        };

        let unwraps = create_replenish_unwraps(&balances, U256::from(10u64));

        assert_eq!(unwraps.len(), 2);
        assert_eq!(unwraps[0].to, dem_wrapper);
        assert_eq!(unwraps[1].to, inf_wrapper);
    }

    fn mock_token_info(token: Address, token_owner: Address, token_type: &str) -> TokenInfo {
        TokenInfo {
            block_number: 0,
            timestamp: 0,
            transaction_index: 0,
            log_index: 0,
            transaction_hash: TxHash::from(fixed_bytes!(
                "0000000000000000000000000000000000000000000000000000000000000000"
            )),
            version: 2,
            info_type: None,
            token_type: token_type.to_string(),
            token,
            token_owner,
        }
    }
}
