use crate::{Sdk, SdkError};
use alloy_primitives::Address;
use circles_types::{PagedResponse, TokenBalanceResponse, TokenHolderRow};

const INFLATIONARY_WRAPPER_TYPE: &str = "CrcV2_ERC20WrapperDeployed_Inflationary";

pub(crate) fn static_wrapped_token_totals(
    balances: Vec<TokenBalanceResponse>,
) -> Vec<TokenBalanceResponse> {
    balances
        .into_iter()
        .filter(|balance| balance.token_type.as_deref() == Some(INFLATIONARY_WRAPPER_TYPE))
        .collect()
}

/// Borrowed tokens facade mirroring the TypeScript `sdk.tokens.*` namespace.
pub struct Tokens<'a> {
    sdk: &'a Sdk,
}

impl<'a> Tokens<'a> {
    pub(crate) fn new(sdk: &'a Sdk) -> Self {
        Self { sdk }
    }

    /// Get the inflationary wrapper address for a Circles token.
    pub async fn get_inflationary_wrapper(&self, token: Address) -> Result<Address, SdkError> {
        self.sdk.inflationary_wrapper(token).await
    }

    /// Get the demurraged wrapper address for a Circles token.
    pub async fn get_demurraged_wrapper(&self, token: Address) -> Result<Address, SdkError> {
        self.sdk.demurraged_wrapper(token).await
    }

    /// Get holders for a token via the native paged RPC endpoint.
    pub async fn get_holders(
        &self,
        token: Address,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<PagedResponse<TokenHolderRow>, SdkError> {
        self.sdk.token_holders(token, limit, cursor).await
    }

    /// Get static wrapped-token totals for inflationary ERC20 wrappers held by a sender.
    pub async fn get_static_wrapped_token_totals_from_sender(
        &self,
        sender: Address,
    ) -> Result<Vec<TokenBalanceResponse>, SdkError> {
        self.sdk
            .static_wrapped_token_totals_from_sender(sender)
            .await
    }

    /// Get redeemable collateral amount for a group/collateral pair.
    pub async fn get_redeemable_amount(
        &self,
        group: Address,
        collateral: Address,
    ) -> Result<alloy_primitives::U256, SdkError> {
        self.sdk.redeemable_amount(group, collateral).await
    }
}

#[cfg(test)]
mod tests {
    use super::{Tokens, static_wrapped_token_totals};
    use crate::config;
    use alloy_primitives::{U256, address};
    use circles_types::{Balance, TokenBalanceResponse};

    #[test]
    fn tokens_facade_is_constructible() {
        let sdk = crate::Sdk::new(config::gnosis_mainnet(), None).expect("sdk");
        let _ = Tokens::new(&sdk);
    }

    #[test]
    fn static_wrapped_token_totals_keep_only_inflationary_wrappers() {
        let inflationary = TokenBalanceResponse {
            token_address: address!("1111111111111111111111111111111111111111"),
            token_id: address!("1111111111111111111111111111111111111111"),
            balance: Balance::Raw(U256::from(7u64)),
            static_atto_circles: Some(U256::from(11u64)),
            static_circles: None,
            token_type: Some("CrcV2_ERC20WrapperDeployed_Inflationary".to_string()),
            version: Some(2),
            atto_circles: Some(U256::from(13u64)),
            circles: None,
            atto_crc: None,
            crc: None,
            is_erc20: true,
            is_erc1155: false,
            is_wrapped: true,
            is_inflationary: true,
            is_group: false,
            token_owner: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        };
        let demurraged = TokenBalanceResponse {
            token_address: address!("2222222222222222222222222222222222222222"),
            token_id: address!("2222222222222222222222222222222222222222"),
            balance: Balance::Raw(U256::from(17u64)),
            static_atto_circles: None,
            static_circles: None,
            token_type: Some("CrcV2_ERC20WrapperDeployed_Demurraged".to_string()),
            version: Some(2),
            atto_circles: Some(U256::from(19u64)),
            circles: None,
            atto_crc: None,
            crc: None,
            is_erc20: true,
            is_erc1155: false,
            is_wrapped: true,
            is_inflationary: false,
            is_group: false,
            token_owner: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
        };

        let totals = static_wrapped_token_totals(vec![inflationary.clone(), demurraged]);

        assert_eq!(totals.len(), 1);
        assert_eq!(totals[0].token_address, inflationary.token_address);
        assert_eq!(totals[0].static_atto_circles, Some(U256::from(11u64)));
    }
}
