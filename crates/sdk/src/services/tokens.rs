use crate::{Sdk, SdkError};
use alloy_primitives::Address;
use circles_types::{PagedResponse, TokenHolderRow};

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
}

#[cfg(test)]
mod tests {
    use super::Tokens;
    use crate::config;

    #[test]
    fn tokens_facade_is_constructible() {
        let sdk = crate::Sdk::new(config::gnosis_mainnet(), None).expect("sdk");
        let _ = Tokens::new(&sdk);
    }
}
