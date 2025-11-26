use crate::error::{TransferError, TransfersErrorSource};
use abis::{DemurrageCircles, HubV2, InflationaryCircles, LiftERC20};
use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::SolCall;
use circles_pathfinder::{
    create_flow_matrix, expected_unwrapped_totals, replace_wrapped_tokens,
    token_info_map_from_path, wrapped_totals_from_path,
};
use circles_rpc::CirclesRpc;
use circles_types::{
    AdvancedTransferOptions, CirclesConfig, FindPathParams, TokenBalanceResponse, TransferStep,
};
use circles_utils::converter::atto_circles_to_atto_static_circles;
use std::collections::HashMap;
use std::str::FromStr;

/// Simple transfer transaction representation.
#[derive(Debug, Clone)]
pub struct TransferTx {
    pub to: Address,
    pub data: Bytes,
    pub value: U256,
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

    /// Control whether approval is checked (default: true). If false, approval tx is always included.
    pub fn with_approval_check(mut self, check: bool) -> Self {
        self.check_approval = check;
        self
    }

    /// Construct an advanced transfer.
    /// Returns the list of transactions to execute in order.
    pub async fn construct_advanced_transfer(
        &self,
        from: Address,
        to: Address,
        amount: U256,
        options: Option<AdvancedTransferOptions>,
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

        if has_wrapped && opts.use_wrapped_balances.unwrap_or(false) == false {
            return Err(TransferError::wrapped_tokens_required());
        }

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

    #[doc(hidden)]
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
                    // Unwrap only the amount used in the path, converted to static
                    let ts_hint = if info.timestamp > 0 {
                        Some(info.timestamp)
                    } else {
                        None
                    };
                    let static_amt = atto_circles_to_atto_static_circles(*amount_dem, ts_hint);
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
}

fn truncate_to_six_decimals(amount: U256) -> U256 {
    let unit = U256::from(1_000_000_000_000u64); // 1e12 wei granularity
    (amount / unit) * unit
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
            .erc20Circles(0u8.into(), to_token)
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
            .erc20Circles(1u8.into(), to_token)
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
}

fn needs_approval_blocking<F, Fut>(f: F) -> Option<bool>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Option<bool>>,
{
    futures::executor::block_on(f())
}
