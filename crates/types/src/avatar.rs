use alloy_primitives::{Address, TxHash, U256};
use serde::{Deserialize, Serialize};

/// Avatar type variants
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AvatarType {
    #[serde(rename = "CrcV2_RegisterHuman")]
    CrcV2RegisterHuman,
    #[serde(rename = "CrcV2_RegisterGroup")]
    CrcV2RegisterGroup,
    #[serde(rename = "CrcV2_RegisterOrganization")]
    CrcV2RegisterOrganization,
    #[serde(rename = "CrcV1_Signup")]
    CrcV1Signup,
    #[serde(rename = "CrcV1_OrganizationSignup")]
    CrcV1OrganizationSignup,
}

/// Geographic location coordinates
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeoLocation {
    pub lat: f64,
    pub lng: f64,
}

/// Avatar information
/// Contains basic information about a Circles avatar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarInfo {
    /// The block number of the event
    pub block_number: u64,
    /// The timestamp of the last change to the avatar
    /// Note: May be undefined for some avatars
    pub timestamp: Option<u64>,
    /// The transaction index
    pub transaction_index: u32,
    /// The log index
    pub log_index: u32,
    /// The hash of the transaction that last changed the avatar
    pub transaction_hash: TxHash,
    /// If the avatar is currently active in version 1 or 2
    /// Note: An avatar that's active in v2 can still have a v1 token. See `has_v1` and `v1_token`.
    pub version: u32,
    /// The type of the avatar
    #[serde(rename = "type")]
    pub avatar_type: AvatarType,
    /// The address of the avatar
    pub avatar: Address,
    /// The personal or group token address (v1) or tokenId (v2)
    /// Note: v1 tokens are erc20 and have a token address. v2 tokens are erc1155 and have a tokenId.
    ///       The v2 tokenId is always an encoded version of the avatar address.
    pub token_id: Option<U256>,
    /// If the avatar is signed up at v1
    pub has_v1: bool,
    /// If the avatar has a v1 token, this is the token address
    pub v1_token: Option<Address>,
    /// The bytes of the avatar's metadata cidv0
    pub cid_v0_digest: Option<String>,
    /// The CIDv0 of the avatar's metadata (profile)
    pub cid_v0: Option<String>,
    /// If the avatar is stopped in v1
    /// Note: This is only set during Avatar initialization.
    pub v1_stopped: Option<bool>,
    /// Indicates whether the entity is a human
    pub is_human: bool,
    /// Groups have a name
    pub name: Option<String>,
    /// Groups have a symbol
    pub symbol: Option<String>,
}

/// Profile information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub description: Option<String>,
    pub preview_image_url: Option<String>,
    pub image_url: Option<String>,
    pub location: Option<String>,
    pub geo_location: Option<GeoLocation>,
    pub extensions: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Group profile with additional symbol field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupProfile {
    #[serde(flatten)]
    pub profile: Profile,
    pub symbol: String,
}

#[cfg(test)]
mod tests {
    use super::Profile;

    #[test]
    fn profile_deserializes_camel_case() {
        let json = r#"{
            "name": "franco",
            "previewImageUrl": "data:image/jpeg;base64,abc",
            "imageUrl": "https://example.com/full.jpg",
            "location": "Berlin"
        }"#;

        let profile: Profile = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(profile.name, "franco");
        assert_eq!(
            profile.preview_image_url.as_deref(),
            Some("data:image/jpeg;base64,abc")
        );
        assert_eq!(
            profile.image_url.as_deref(),
            Some("https://example.com/full.jpg")
        );
        assert_eq!(profile.location.as_deref(), Some("Berlin"));
    }
}
