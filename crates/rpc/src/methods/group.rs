use crate::client::RpcClient;
use crate::error::Result;
use circles_types::{GroupMembershipRow, GroupRow};

/// Methods for group membership lookups.
#[derive(Clone, Debug)]
pub struct GroupMethods {
    client: RpcClient,
}

impl GroupMethods {
    /// Create a new accessor for group-related RPCs.
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// circles_getGroupMemberships
    pub async fn get_memberships(
        &self,
        avatar: circles_types::Address,
    ) -> Result<Vec<GroupMembershipRow>> {
        self.client
            .call("circles_getGroupMemberships", (avatar,))
            .await
    }

    /// circles_getGroups
    pub async fn get_groups(&self, avatar: circles_types::Address) -> Result<Vec<GroupRow>> {
        self.client.call("circles_getGroups", (avatar,)).await
    }
}
