use crate::client::RpcClient;
use crate::error::Result;
use circles_types::TableInfo;

/// Methods for table/schema introspection.
#[derive(Clone, Debug)]
pub struct TablesMethods {
    client: RpcClient,
}

impl TablesMethods {
    /// Create a new accessor for table listing RPCs.
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// circles_tables
    pub async fn tables(&self) -> Result<Vec<TableInfo>> {
        self.client.call("circles_tables", ()).await
    }
}
