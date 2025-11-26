use crate::client::RpcClient;
use crate::error::Result;
use circles_types::Profile;

/// Methods for full-text profile search.
#[derive(Clone, Debug)]
pub struct SearchMethods {
    client: RpcClient,
}

impl SearchMethods {
    /// Create a new accessor for search RPCs.
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// circles_searchProfiles
    pub async fn search_profiles(&self, query: String, limit: Option<u32>) -> Result<Vec<Profile>> {
        self.client
            .call("circles_searchProfiles", (query, limit))
            .await
    }
}
