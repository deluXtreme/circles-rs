use crate::client::RpcClient;
use crate::error::Result;
use alloy_primitives::U256;
use circles_types::{Address, TokenBalanceResponse, TokenHolder};
use std::str::FromStr;

/// Methods for token balance and holder lookups.
///
/// `get_token_balances` selects v1/v2 via `use_v2`; holders come back as
/// demurraged totals and are normalized to `U256`.
#[derive(Clone, Debug)]
pub struct TokenMethods {
    client: RpcClient,
}

/// Normalized token holder with numeric balance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenHolderNormalized {
    /// Account that owns the token balance.
    pub account: Address,
    /// The token address.
    pub token_address: Address,
    /// Balance as `U256` (demurraged).
    pub balance: U256,
}

impl From<TokenHolder> for TokenHolderNormalized {
    fn from(holder: TokenHolder) -> Self {
        let balance = U256::from_str(&holder.demurraged_total_balance).unwrap_or_default();
        Self {
            account: holder.account,
            token_address: holder.token_address,
            balance,
        }
    }
}

impl TokenMethods {
    /// Create a new accessor for token-related RPCs.
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// circles_getTokenBalances (v1/v2, selected via `use_v2`)
    pub async fn get_token_balances(
        &self,
        address: Address,
        as_time_circles: bool,
        use_v2: bool,
    ) -> Result<Vec<TokenBalanceResponse>> {
        let method = if use_v2 {
            "circlesV2_getTokenBalances"
        } else {
            "circles_getTokenBalances"
        };
        self.client.call(method, (address, as_time_circles)).await
    }

    /// circles_getTokenHolders (currently assumed v2) returning normalized balances by default.
    pub async fn get_token_holders(&self, token: Address) -> Result<Vec<TokenHolderNormalized>> {
        let holders: Vec<TokenHolder> = self
            .client
            .call("circles_getTokenHolders", (token,))
            .await?;
        Ok(holders
            .into_iter()
            .map(TokenHolderNormalized::from)
            .collect())
    }

    /// Raw token holders (string balances) if needed.
    pub async fn get_token_holders_raw(&self, token: Address) -> Result<Vec<TokenHolder>> {
        self.client.call("circles_getTokenHolders", (token,)).await
    }
}
