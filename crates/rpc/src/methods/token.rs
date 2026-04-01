use crate::client::RpcClient;
use crate::error::Result;
use alloy_primitives::U256;
use circles_types::{Address, PagedResponse, TokenBalanceResponse, TokenHolder, TokenHolderRow};
use std::str::FromStr;

const DEFAULT_TOKEN_HOLDER_LIMIT: u32 = 100;

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

impl From<TokenHolderRow> for TokenHolderNormalized {
    fn from(holder: TokenHolderRow) -> Self {
        let balance = U256::from_str(&holder.balance).unwrap_or_default();
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

    /// Native paged token holders via `circles_getTokenHolders`.
    pub async fn get_token_holders_page(
        &self,
        token: Address,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<PagedResponse<TokenHolderRow>> {
        self.client
            .call(
                "circles_getTokenHolders",
                (
                    token,
                    limit.unwrap_or(DEFAULT_TOKEN_HOLDER_LIMIT),
                    cursor.map(str::to_owned),
                ),
            )
            .await
    }

    /// circles_getTokenHolders (currently assumed v2) returning normalized balances by default.
    pub async fn get_token_holders(&self, token: Address) -> Result<Vec<TokenHolderNormalized>> {
        Ok(self
            .get_token_holders_raw(token)
            .await?
            .into_iter()
            .map(TokenHolderNormalized::from)
            .collect())
    }

    /// Raw token holders (string balances) if needed.
    pub async fn get_token_holders_raw(&self, token: Address) -> Result<Vec<TokenHolder>> {
        let mut cursor: Option<String> = None;
        let mut holders = Vec::new();

        loop {
            let page = self
                .get_token_holders_page(token, Some(DEFAULT_TOKEN_HOLDER_LIMIT), cursor.as_deref())
                .await?;

            holders.extend(page.results.into_iter().map(|holder| TokenHolder {
                account: holder.account,
                token_address: holder.token_address,
                demurraged_total_balance: holder.balance,
            }));

            if !page.has_more {
                break;
            }
            cursor = page.next_cursor;
            if cursor.is_none() {
                break;
            }
        }

        Ok(holders)
    }
}
