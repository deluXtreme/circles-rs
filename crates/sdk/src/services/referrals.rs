use crate::core::Core;
use alloy_primitives::{Address, keccak256};
use circles_abis::ReferralsModule;
use k256::{SecretKey, elliptic_curve::rand_core::OsRng, elliptic_curve::sec1::ToEncodedPoint};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

/// Errors surfaced by the optional referrals backend client.
#[derive(Debug, Error)]
pub enum ReferralsError {
    #[error("invalid referrals service url `{url}`: {reason}")]
    InvalidUrl { url: String, reason: String },
    #[error("referrals service url cannot be a base: {url}")]
    CannotBeABase { url: String },
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("referrals store failed: {0}")]
    StoreFailed(String),
    #[error("referrals batch store failed: {0}")]
    StoreBatchFailed(String),
    #[error("failed to retrieve referral: {reason}")]
    RetrieveFailed {
        reason: String,
        http_status: Option<StatusCode>,
    },
    #[error("failed to list referrals: {0}")]
    ListFailed(String),
    #[error("authentication required to list referrals")]
    AuthRequired,
    #[error("unexpected response format (status {status}): {body}")]
    DecodeFailed { status: StatusCode, body: String },
    #[error("invalid referral private key: {0}")]
    InvalidPrivateKey(String),
    #[error("referrals contract call failed: {0}")]
    Contract(String),
}

/// Referral status lifecycle exposed by the referrals backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReferralStatus {
    Pending,
    Stale,
    Confirmed,
    Claimed,
    Expired,
}

/// Referral info returned from the public retrieve endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferralInfo {
    pub inviter: Option<String>,
    pub status: Option<ReferralStatus>,
    pub account_address: Option<String>,
    pub error: Option<String>,
}

/// Distribution-session metadata attached to private referrals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferralSession {
    pub id: String,
    pub slug: String,
    pub label: Option<String>,
}

/// Full private referral record from the authenticated backend view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Referral {
    pub id: String,
    pub private_key: String,
    pub status: ReferralStatus,
    pub account_address: Option<String>,
    pub created_at: String,
    pub pending_at: String,
    pub stale_at: Option<String>,
    pub confirmed_at: Option<String>,
    pub claimed_at: Option<String>,
    pub sessions: Vec<ReferralSession>,
}

/// Paginated authenticated referral list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferralList {
    pub referrals: Vec<Referral>,
    pub count: u32,
    pub total: u32,
    pub limit: u32,
    pub offset: u32,
}

/// Public masked referral preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferralPreview {
    pub id: String,
    pub key_preview: String,
    pub status: ReferralStatus,
    pub account_address: Option<String>,
    pub created_at: String,
    pub pending_at: Option<String>,
    pub stale_at: Option<String>,
    pub confirmed_at: Option<String>,
    pub claimed_at: Option<String>,
    pub in_session: bool,
}

/// Cache freshness metadata for public preview queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReferralSyncStatus {
    Synced,
    Cached,
}

/// Paginated public referral preview list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferralPreviewList {
    pub referrals: Vec<ReferralPreview>,
    pub count: u32,
    pub total: u32,
    pub limit: u32,
    pub offset: u32,
    pub sync_status: ReferralSyncStatus,
}

/// Error payload returned by the referrals backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
}

/// One item in the batch store request body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferralStoreInput {
    pub private_key: String,
    pub inviter: Address,
}

/// Error entry returned from the batch store endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreBatchError {
    pub index: u32,
    pub key_preview: String,
    pub reason: String,
}

/// Batch store result returned by the referrals backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreBatchResult {
    pub success: bool,
    pub stored: u32,
    pub failed: u32,
    pub errors: Option<Vec<StoreBatchError>>,
}

/// Optional filters for authenticated `my-referrals` queries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReferralListMineOptions {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub in_session: Option<bool>,
    pub status: Option<String>,
}

/// Optional filters for public `list/{address}` queries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReferralPublicListOptions {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub in_session: Option<bool>,
}

/// Optional referrals backend client.
#[derive(Clone)]
pub struct Referrals {
    base_url: Url,
    client: Client,
    core: Arc<Core>,
}

impl Referrals {
    pub fn new(
        referrals_service_url: impl AsRef<str>,
        core: Arc<Core>,
    ) -> Result<Self, ReferralsError> {
        Self::with_client(referrals_service_url, core, Client::new())
    }

    pub fn with_client(
        referrals_service_url: impl AsRef<str>,
        core: Arc<Core>,
        client: Client,
    ) -> Result<Self, ReferralsError> {
        let base_url = normalize_base_url(referrals_service_url.as_ref())?;
        Ok(Self {
            base_url,
            client,
            core,
        })
    }

    pub async fn store(&self, private_key: &str, inviter: Address) -> Result<(), ReferralsError> {
        let url = endpoint(&self.base_url, "store")?;
        let response = self
            .client
            .post(url)
            .json(&ReferralStoreInput {
                private_key: private_key.to_owned(),
                inviter,
            })
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ReferralsError::StoreFailed(api_reason(status, &body)));
        }

        Ok(())
    }

    pub async fn store_batch(
        &self,
        invitations: &[ReferralStoreInput],
    ) -> Result<StoreBatchResult, ReferralsError> {
        let url = endpoint(&self.base_url, "store-batch")?;
        let response = self
            .client
            .post(url)
            .json(&StoreBatchRequest { invitations })
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ReferralsError::StoreBatchFailed(api_reason(status, &body)));
        }

        serde_json::from_str(&body).map_err(|_| ReferralsError::DecodeFailed { status, body })
    }

    pub async fn retrieve(&self, private_key: &str) -> Result<ReferralInfo, ReferralsError> {
        let signer = private_key_to_address(private_key)?;
        let ReferralsModule::accountsReturn { account, claimed } = self
            .core
            .referrals_module()
            .accounts(signer)
            .call()
            .await
            .map_err(|e| ReferralsError::Contract(e.to_string()))?;

        let mut url = endpoint(&self.base_url, "retrieve")?;
        url.query_pairs_mut().append_pair("key", private_key);

        let response = self.client.get(url).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if status.is_success() || status == StatusCode::GONE || claimed {
            let mut info: ReferralInfo =
                serde_json::from_str(&body).map_err(|_| ReferralsError::DecodeFailed {
                    status,
                    body: body.clone(),
                })?;
            if account == Address::ZERO {
                info = referral_not_found_info(signer, Some(info));
            }
            return Ok(info);
        }

        if account == Address::ZERO {
            return Ok(referral_not_found_info(signer, None));
        }

        Err(ReferralsError::RetrieveFailed {
            reason: api_reason(status, &body),
            http_status: Some(status),
        })
    }

    pub async fn list_mine(
        &self,
        auth_token: Option<&str>,
        opts: Option<ReferralListMineOptions>,
    ) -> Result<ReferralList, ReferralsError> {
        let token = auth_token.ok_or(ReferralsError::AuthRequired)?;
        let mut url = endpoint(&self.base_url, "my-referrals")?;
        append_mine_query(&mut url, opts.as_ref());

        let response = self.client.get(url).bearer_auth(token).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ReferralsError::ListFailed(api_reason(status, &body)));
        }

        serde_json::from_str(&body).map_err(|_| ReferralsError::DecodeFailed { status, body })
    }

    pub async fn list_public(
        &self,
        inviter: Address,
        opts: Option<ReferralPublicListOptions>,
    ) -> Result<ReferralPreviewList, ReferralsError> {
        let url = public_list_url(&self.base_url, inviter, opts.as_ref())?;
        let response = self.client.get(url).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ReferralsError::ListFailed(api_reason(status, &body)));
        }

        serde_json::from_str(&body).map_err(|_| ReferralsError::DecodeFailed { status, body })
    }
}

pub fn generate_private_key() -> String {
    let secret = SecretKey::random(&mut OsRng);
    format!("0x{}", hex::encode(secret.to_bytes()))
}

pub fn private_key_to_address(private_key: &str) -> Result<Address, ReferralsError> {
    let clean_key = private_key.strip_prefix("0x").unwrap_or(private_key);
    let key_bytes =
        hex::decode(clean_key).map_err(|err| ReferralsError::InvalidPrivateKey(err.to_string()))?;
    let secret = SecretKey::from_slice(&key_bytes)
        .map_err(|err| ReferralsError::InvalidPrivateKey(err.to_string()))?;
    let public_key = secret.public_key();
    let encoded = public_key.to_encoded_point(false);
    let hash = keccak256(&encoded.as_bytes()[1..]);
    Ok(Address::from_slice(&hash[12..]))
}

#[derive(Debug, Serialize)]
struct StoreBatchRequest<'a> {
    invitations: &'a [ReferralStoreInput],
}

fn endpoint(base: &Url, path: &str) -> Result<Url, ReferralsError> {
    base.join(path)
        .map_err(|source| ReferralsError::InvalidUrl {
            url: format!("{base}{path}"),
            reason: source.to_string(),
        })
}

fn public_list_url(
    base: &Url,
    inviter: Address,
    opts: Option<&ReferralPublicListOptions>,
) -> Result<Url, ReferralsError> {
    let mut url = endpoint(base, &format!("list/{inviter:#x}"))?;
    append_public_query(&mut url, opts);
    Ok(url)
}

fn append_mine_query(url: &mut Url, opts: Option<&ReferralListMineOptions>) {
    let Some(opts) = opts else {
        return;
    };

    let mut pairs = url.query_pairs_mut();
    if let Some(limit) = opts.limit {
        pairs.append_pair("limit", &limit.to_string());
    }
    if let Some(offset) = opts.offset {
        pairs.append_pair("offset", &offset.to_string());
    }
    if let Some(in_session) = opts.in_session {
        pairs.append_pair("inSession", if in_session { "true" } else { "false" });
    }
    if let Some(status) = opts.status.as_deref() {
        pairs.append_pair("status", status);
    }
}

fn append_public_query(url: &mut Url, opts: Option<&ReferralPublicListOptions>) {
    let Some(opts) = opts else {
        return;
    };

    let mut pairs = url.query_pairs_mut();
    if let Some(limit) = opts.limit {
        pairs.append_pair("limit", &limit.to_string());
    }
    if let Some(offset) = opts.offset {
        pairs.append_pair("offset", &offset.to_string());
    }
    if let Some(in_session) = opts.in_session {
        pairs.append_pair("inSession", if in_session { "true" } else { "false" });
    }
}

fn api_reason(status: StatusCode, body: &str) -> String {
    serde_json::from_str::<ApiError>(body)
        .map(|err| err.error)
        .unwrap_or_else(|_| {
            if body.is_empty() {
                status
                    .canonical_reason()
                    .unwrap_or("request failed")
                    .to_string()
            } else {
                body.to_string()
            }
        })
}

fn normalize_base_url(raw: &str) -> Result<Url, ReferralsError> {
    let mut url = Url::parse(raw).map_err(|source| ReferralsError::InvalidUrl {
        url: raw.to_owned(),
        reason: source.to_string(),
    })?;

    if !url.path().ends_with('/') {
        url.path_segments_mut()
            .map_err(|_| ReferralsError::CannotBeABase {
                url: raw.to_owned(),
            })?
            .push("");
    }

    Ok(url)
}

fn referral_not_found_info(signer: Address, info: Option<ReferralInfo>) -> ReferralInfo {
    let mut info = info.unwrap_or(ReferralInfo {
        inviter: None,
        status: None,
        account_address: None,
        error: None,
    });
    info.error = Some(format!(
        "Referral not found on-chain for signer {signer:#x}"
    ));
    info
}

#[cfg(test)]
mod tests {
    use super::{
        ReferralInfo, ReferralListMineOptions, ReferralPublicListOptions, api_reason,
        normalize_base_url, private_key_to_address, referral_not_found_info,
    };
    use alloy_primitives::{Address, address};
    use reqwest::StatusCode;

    #[test]
    fn ensures_trailing_slash() {
        let normalized = normalize_base_url("https://example.com/api").unwrap();
        assert_eq!(normalized.as_str(), "https://example.com/api/");
    }

    #[test]
    fn keeps_existing_slash() {
        let normalized = normalize_base_url("https://example.com/api/").unwrap();
        assert_eq!(normalized.as_str(), "https://example.com/api/");
    }

    #[test]
    fn private_key_to_address_matches_anvil_fixture() {
        let signer = private_key_to_address(
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        )
        .unwrap();

        assert_eq!(signer, address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266"));
    }

    #[test]
    fn referral_not_found_info_sets_ts_style_error() {
        let signer = Address::repeat_byte(0x11);
        let info = referral_not_found_info(
            signer,
            Some(ReferralInfo {
                inviter: Some("0xabc".into()),
                status: None,
                account_address: None,
                error: None,
            }),
        );

        assert_eq!(info.inviter.as_deref(), Some("0xabc"));
        assert_eq!(
            info.error.as_deref(),
            Some(
                "Referral not found on-chain for signer 0x1111111111111111111111111111111111111111"
            )
        );
    }

    #[test]
    fn api_reason_prefers_backend_error_field() {
        assert_eq!(
            api_reason(StatusCode::BAD_REQUEST, r#"{"error":"bad key"}"#),
            "bad key"
        );
    }

    #[test]
    fn options_default_shapes_are_empty() {
        let public = ReferralPublicListOptions::default();
        let mine = ReferralListMineOptions::default();
        assert!(public.limit.is_none());
        assert!(mine.status.is_none());
    }
}
