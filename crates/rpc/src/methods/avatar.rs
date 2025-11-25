use crate::client::RpcClient;
use crate::error::Result;
use circles_types::{Address, AvatarInfo, Profile};

/// Methods for avatar/profile lookups.
#[derive(Clone, Debug)]
pub struct AvatarMethods {
    client: RpcClient,
}

impl AvatarMethods {
    /// Create a new accessor for avatar/profile RPCs.
    pub fn new(client: RpcClient) -> Self {
        Self { client }
    }

    /// circles_getAvatarInfo
    pub async fn get_avatar_info(&self, address: Address) -> Result<AvatarInfo> {
        self.client.call("circles_getAvatarInfo", (address,)).await
    }

    /// circles_getAvatarInfoBatch
    pub async fn get_avatar_info_batch(&self, addresses: Vec<Address>) -> Result<Vec<AvatarInfo>> {
        self.client
            .call("circles_getAvatarInfoBatch", (addresses,))
            .await
    }

    /// circles_getProfileCid
    pub async fn get_profile_cid(&self, address: Address) -> Result<String> {
        self.client.call("circles_getProfileCid", (address,)).await
    }

    /// circles_getProfileCidBatch
    pub async fn get_profile_cid_batch(&self, addresses: Vec<Address>) -> Result<Vec<String>> {
        self.client
            .call("circles_getProfileCidBatch", (addresses,))
            .await
    }

    /// circles_getProfileByCid
    pub async fn get_profile_by_cid(&self, cid: String) -> Result<Profile> {
        self.client.call("circles_getProfileByCid", (cid,)).await
    }

    /// circles_getProfileByCidBatch
    pub async fn get_profile_by_cid_batch(&self, cids: Vec<String>) -> Result<Vec<Profile>> {
        self.client
            .call("circles_getProfileByCidBatch", (cids,))
            .await
    }

    /// circles_getProfileByAddress
    pub async fn get_profile_by_address(&self, address: Address) -> Result<Profile> {
        self.client
            .call("circles_getProfileByAddress", (address,))
            .await
    }

    /// circles_getProfileByAddressBatch
    pub async fn get_profile_by_address_batch(
        &self,
        addresses: Vec<Address>,
    ) -> Result<Vec<Profile>> {
        self.client
            .call("circles_getProfileByAddressBatch", (addresses,))
            .await
    }
}
