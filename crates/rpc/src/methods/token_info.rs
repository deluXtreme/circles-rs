use crate::client::RpcClient;
use crate::error::Result;
use circles_types::{Address, TokenInfo};

/// Methods for retrieving token metadata (`circles_getTokenInfo` + batch).
#[derive(Clone, Debug)]
pub struct TokenInfoMethods {
    client: RpcClient,
}

impl TokenInfoMethods {
    /// Create a new accessor for token metadata RPCs.
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// circles_getTokenInfo
    pub async fn get_token_info(&self, token: Address) -> Result<TokenInfo> {
        self.client.call("circles_getTokenInfo", (token,)).await
    }

    /// circles_getTokenInfoBatch
    pub async fn get_token_info_batch(&self, tokens: Vec<Address>) -> Result<Vec<TokenInfo>> {
        self.client
            .call("circles_getTokenInfoBatch", (tokens,))
            .await
    }
}
