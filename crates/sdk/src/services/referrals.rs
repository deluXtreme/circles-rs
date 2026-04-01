use crate::core::Core;
use alloy_primitives::{Address, keccak256};
use circles_abis::ReferralsModule;
use k256::{SecretKey, elliptic_curve::rand_core::OsRng, elliptic_curve::sec1::ToEncodedPoint};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
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
    #[error("distribution session request failed ({code:?}, status {http_status}): {reason}")]
    SessionFailed {
        reason: String,
        code: SessionErrorCode,
        http_status: StatusCode,
    },
    #[error("distribution dispense failed ({code:?}, status {http_status}): {reason}")]
    DispenseFailed {
        reason: String,
        code: DispenseErrorCode,
        http_status: StatusCode,
    },
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

/// Referral distribution-session metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionSession {
    pub id: String,
    pub slug: String,
    pub inviter_address: Address,
    pub label: Option<String>,
    pub quota: u32,
    pub dispensed_count: u32,
    pub expires_at: Option<String>,
    pub paused: bool,
    pub created_at: String,
    pub updated_at: String,
    pub distribution_url: Option<String>,
}

/// Paginated distribution-session list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionSessionList {
    pub sessions: Vec<DistributionSession>,
    pub total: u32,
    pub limit: u32,
    pub offset: u32,
}

/// Parameters for creating a new distribution session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionParams {
    pub inviter_address: Address,
    pub quota: u32,
    pub label: Option<String>,
    pub expires_at: Option<String>,
}

/// Parameters for updating an existing distribution session.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSessionParams {
    pub label: Option<String>,
    pub quota: Option<u32>,
    pub expires_at: Option<String>,
    pub paused: Option<bool>,
}

/// Optional pagination for distribution-session listing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DistributionSessionListOptions {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Distribution-session key lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionKeyStatus {
    Queued,
    Dispatched,
    Claimed,
}

/// Single key entry within a distribution session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionKey {
    pub id: String,
    pub private_key: String,
    pub signer_address: Option<Address>,
    pub account_address: Option<Address>,
    pub status: SessionKeyStatus,
    pub dispatched_at: Option<String>,
    pub claimed_at: Option<String>,
    pub added_at: String,
}

/// Paginated key list for a distribution session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionKeyList {
    pub keys: Vec<SessionKey>,
    pub total: u32,
    pub queued_count: u32,
    pub dispatched_count: u32,
    pub claimed_count: u32,
    pub limit: u32,
    pub offset: u32,
}

/// Optional pagination for session-key listing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionKeyListOptions {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Per-key error entry returned when adding keys to a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddKeysError {
    pub key: String,
    pub error: String,
}

/// Result returned by the session key-addition endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddKeysResult {
    pub added: u32,
    pub skipped: u32,
    pub claimed: u32,
    pub errors: Vec<AddKeysError>,
}

/// Result returned when dispensing a key through a public session slug.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DispenseResult {
    pub private_key: String,
    pub inviter: Address,
    pub claim_url: Option<String>,
    pub session_slug: String,
}

/// Typed error codes for distribution-session CRUD operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionErrorCode {
    ValidationError,
    NotFound,
    Conflict,
    ServerError,
}

/// Typed error codes for public key dispensing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DispenseErrorCode {
    SessionNotFound,
    PoolEmpty,
    SessionExpired,
    QuotaExhausted,
    SessionPaused,
    RateLimited,
    Unknown,
}

/// Async token future returned by a referrals auth-token provider.
pub type ReferralsAuthTokenFuture =
    Pin<Box<dyn Future<Output = Result<String, ReferralsError>> + Send>>;

/// Async bearer-token source for authenticated referrals backend calls.
pub type ReferralsAuthTokenProvider = Arc<dyn Fn() -> ReferralsAuthTokenFuture + Send + Sync>;

/// Optional referrals backend client.
#[derive(Clone)]
pub struct Referrals {
    base_url: Url,
    client: Client,
    core: Arc<Core>,
    auth_token_provider: Option<ReferralsAuthTokenProvider>,
}

/// Optional referral distribution-session backend client.
#[derive(Clone)]
pub struct Distributions {
    base_url: Url,
    client: Client,
    auth_token_provider: Option<ReferralsAuthTokenProvider>,
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
            auth_token_provider: None,
        })
    }

    /// Clone this client and use a fixed bearer token for authenticated calls.
    pub fn with_auth_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token_provider = Some(static_auth_token_provider(token.into()));
        self
    }

    /// Clone this client and use an async bearer-token provider for authenticated calls.
    pub fn with_auth_token_provider<F, Fut>(mut self, provider: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<String, ReferralsError>> + Send + 'static,
    {
        self.auth_token_provider = Some(Arc::new(move || Box::pin(provider())));
        self
    }

    /// Construct a distribution-session client that reuses this referrals client's auth setup.
    pub fn distributions(&self) -> Distributions {
        Distributions::from_parts(
            self.base_url.clone(),
            self.client.clone(),
            self.auth_token_provider.clone(),
        )
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
        let token = self.resolve_auth_token(auth_token).await?;
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

    /// TypeScript-style authenticated referral listing using the configured token source.
    pub async fn list_mine_authenticated(
        &self,
        opts: Option<ReferralListMineOptions>,
    ) -> Result<ReferralList, ReferralsError> {
        self.list_mine(None, opts).await
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

    async fn resolve_auth_token(&self, explicit: Option<&str>) -> Result<String, ReferralsError> {
        resolve_auth_token(self.auth_token_provider.as_ref(), explicit).await
    }
}

impl Distributions {
    pub fn new(referrals_service_url: impl AsRef<str>) -> Result<Self, ReferralsError> {
        Self::with_client(referrals_service_url, Client::new())
    }

    pub fn with_client(
        referrals_service_url: impl AsRef<str>,
        client: Client,
    ) -> Result<Self, ReferralsError> {
        let base_url = normalize_base_url(referrals_service_url.as_ref())?;
        Ok(Self::from_parts(base_url, client, None))
    }

    fn from_parts(
        base_url: Url,
        client: Client,
        auth_token_provider: Option<ReferralsAuthTokenProvider>,
    ) -> Self {
        Self {
            base_url,
            client,
            auth_token_provider,
        }
    }

    /// Clone this client and use a fixed bearer token for authenticated calls.
    pub fn with_auth_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token_provider = Some(static_auth_token_provider(token.into()));
        self
    }

    /// Clone this client and use an async bearer-token provider for authenticated calls.
    pub fn with_auth_token_provider<F, Fut>(mut self, provider: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<String, ReferralsError>> + Send + 'static,
    {
        self.auth_token_provider = Some(Arc::new(move || Box::pin(provider())));
        self
    }

    pub async fn create_session(
        &self,
        params: &CreateSessionParams,
    ) -> Result<DistributionSession, ReferralsError> {
        let token = self.resolve_auth_token().await?;
        let url = endpoint(&self.base_url, "distributions/sessions")?;
        let response = self
            .client
            .post(url)
            .bearer_auth(token)
            .json(params)
            .send()
            .await?;
        decode_session_response(response, "create session").await
    }

    pub async fn list_sessions(
        &self,
        inviter: Address,
        opts: Option<&DistributionSessionListOptions>,
    ) -> Result<DistributionSessionList, ReferralsError> {
        let token = self.resolve_auth_token().await?;
        let url = session_collection_url(&self.base_url, inviter, opts)?;
        let response = self.client.get(url).bearer_auth(token).send().await?;
        decode_session_list_response(response, "list sessions").await
    }

    pub async fn get_session(&self, id: &str) -> Result<DistributionSession, ReferralsError> {
        let token = self.resolve_auth_token().await?;
        let url = session_url(&self.base_url, id)?;
        let response = self.client.get(url).bearer_auth(token).send().await?;
        decode_session_response(response, "get session").await
    }

    pub async fn update_session(
        &self,
        id: &str,
        params: &UpdateSessionParams,
    ) -> Result<DistributionSession, ReferralsError> {
        let token = self.resolve_auth_token().await?;
        let url = session_url(&self.base_url, id)?;
        let response = self
            .client
            .patch(url)
            .bearer_auth(token)
            .json(params)
            .send()
            .await?;
        decode_session_response(response, "update session").await
    }

    pub async fn delete_session(&self, id: &str) -> Result<(), ReferralsError> {
        let token = self.resolve_auth_token().await?;
        let url = session_url(&self.base_url, id)?;
        let response = self.client.delete(url).bearer_auth(token).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ReferralsError::SessionFailed {
                reason: session_error_reason(status, &body, "delete session"),
                code: session_error_code(status),
                http_status: status,
            });
        }

        Ok(())
    }

    pub async fn add_keys(
        &self,
        id: &str,
        keys: &[String],
    ) -> Result<AddKeysResult, ReferralsError> {
        let token = self.resolve_auth_token().await?;
        let url = session_keys_url(&self.base_url, id, None)?;
        let response = self
            .client
            .post(url)
            .bearer_auth(token)
            .json(&AddKeysRequest { keys })
            .send()
            .await?;
        decode_add_keys_response(response).await
    }

    pub async fn list_keys(
        &self,
        id: &str,
        opts: Option<&SessionKeyListOptions>,
    ) -> Result<SessionKeyList, ReferralsError> {
        let token = self.resolve_auth_token().await?;
        let url = session_keys_url(&self.base_url, id, opts)?;
        let response = self.client.get(url).bearer_auth(token).send().await?;
        decode_session_keys_response(response, "list keys").await
    }

    pub async fn remove_key(&self, id: &str, key_id: &str) -> Result<(), ReferralsError> {
        let token = self.resolve_auth_token().await?;
        let url = session_key_url(&self.base_url, id, key_id)?;
        let response = self.client.delete(url).bearer_auth(token).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ReferralsError::SessionFailed {
                reason: session_error_reason(status, &body, "remove key"),
                code: session_error_code(status),
                http_status: status,
            });
        }

        Ok(())
    }

    pub async fn dispense(&self, slug: &str) -> Result<DispenseResult, ReferralsError> {
        let url = dispense_url(&self.base_url, slug)?;
        let response = self
            .client
            .get(url)
            .header("Accept", "application/json")
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            let reason = api_reason(status, &body);
            return Err(ReferralsError::DispenseFailed {
                code: dispense_error_code(status, &reason),
                http_status: status,
                reason,
            });
        }

        serde_json::from_str(&body).map_err(|_| ReferralsError::DecodeFailed { status, body })
    }

    async fn resolve_auth_token(&self) -> Result<String, ReferralsError> {
        resolve_auth_token(self.auth_token_provider.as_ref(), None).await
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

#[derive(Debug, Serialize)]
struct AddKeysRequest<'a> {
    keys: &'a [String],
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

fn session_collection_url(
    base: &Url,
    inviter: Address,
    opts: Option<&DistributionSessionListOptions>,
) -> Result<Url, ReferralsError> {
    let mut url = endpoint(base, "distributions/sessions")?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("inviter", &format!("{inviter:#x}"));
        if let Some(opts) = opts {
            if let Some(limit) = opts.limit {
                pairs.append_pair("limit", &limit.to_string());
            }
            if let Some(offset) = opts.offset {
                pairs.append_pair("offset", &offset.to_string());
            }
        }
    }
    Ok(url)
}

fn session_url(base: &Url, id: &str) -> Result<Url, ReferralsError> {
    endpoint(base, &format!("distributions/sessions/{id}"))
}

fn session_keys_url(
    base: &Url,
    id: &str,
    opts: Option<&SessionKeyListOptions>,
) -> Result<Url, ReferralsError> {
    let mut url = endpoint(base, &format!("distributions/sessions/{id}/keys"))?;
    if let Some(opts) = opts {
        let mut pairs = url.query_pairs_mut();
        if let Some(limit) = opts.limit {
            pairs.append_pair("limit", &limit.to_string());
        }
        if let Some(offset) = opts.offset {
            pairs.append_pair("offset", &offset.to_string());
        }
    }
    Ok(url)
}

fn session_key_url(base: &Url, id: &str, key_id: &str) -> Result<Url, ReferralsError> {
    endpoint(base, &format!("distributions/sessions/{id}/keys/{key_id}"))
}

fn dispense_url(base: &Url, slug: &str) -> Result<Url, ReferralsError> {
    endpoint(base, &format!("d/{slug}"))
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

fn static_auth_token_provider(token: String) -> ReferralsAuthTokenProvider {
    let token = Arc::new(token);
    Arc::new(move || {
        let token = token.clone();
        Box::pin(async move { Ok((*token).clone()) })
    })
}

async fn resolve_auth_token(
    provider: Option<&ReferralsAuthTokenProvider>,
    explicit: Option<&str>,
) -> Result<String, ReferralsError> {
    if let Some(token) = explicit {
        return Ok(token.to_owned());
    }

    let provider = provider.ok_or(ReferralsError::AuthRequired)?;
    provider().await
}

fn session_error_code(status: StatusCode) -> SessionErrorCode {
    match status {
        StatusCode::BAD_REQUEST => SessionErrorCode::ValidationError,
        StatusCode::NOT_FOUND => SessionErrorCode::NotFound,
        StatusCode::CONFLICT => SessionErrorCode::Conflict,
        _ => SessionErrorCode::ServerError,
    }
}

fn session_error_reason(status: StatusCode, body: &str, action: &str) -> String {
    let reason = api_reason(status, body);
    if reason.is_empty() {
        format!("Failed to {action}")
    } else {
        reason
    }
}

fn dispense_error_code(status: StatusCode, message: &str) -> DispenseErrorCode {
    match status {
        StatusCode::NOT_FOUND => {
            if message.contains("keys available") {
                DispenseErrorCode::PoolEmpty
            } else {
                DispenseErrorCode::SessionNotFound
            }
        }
        StatusCode::GONE => {
            if message.contains("quota") {
                DispenseErrorCode::QuotaExhausted
            } else {
                DispenseErrorCode::SessionExpired
            }
        }
        StatusCode::LOCKED => DispenseErrorCode::SessionPaused,
        StatusCode::TOO_MANY_REQUESTS => DispenseErrorCode::RateLimited,
        _ => DispenseErrorCode::Unknown,
    }
}

async fn decode_session_response(
    response: reqwest::Response,
    action: &str,
) -> Result<DistributionSession, ReferralsError> {
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(ReferralsError::SessionFailed {
            reason: session_error_reason(status, &body, action),
            code: session_error_code(status),
            http_status: status,
        });
    }

    serde_json::from_str(&body).map_err(|_| ReferralsError::DecodeFailed { status, body })
}

async fn decode_session_list_response(
    response: reqwest::Response,
    action: &str,
) -> Result<DistributionSessionList, ReferralsError> {
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(ReferralsError::SessionFailed {
            reason: session_error_reason(status, &body, action),
            code: session_error_code(status),
            http_status: status,
        });
    }

    serde_json::from_str(&body).map_err(|_| ReferralsError::DecodeFailed { status, body })
}

async fn decode_session_keys_response(
    response: reqwest::Response,
    action: &str,
) -> Result<SessionKeyList, ReferralsError> {
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(ReferralsError::SessionFailed {
            reason: session_error_reason(status, &body, action),
            code: session_error_code(status),
            http_status: status,
        });
    }

    serde_json::from_str(&body).map_err(|_| ReferralsError::DecodeFailed { status, body })
}

async fn decode_add_keys_response(
    response: reqwest::Response,
) -> Result<AddKeysResult, ReferralsError> {
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(ReferralsError::SessionFailed {
            reason: session_error_reason(status, &body, "add keys"),
            code: session_error_code(status),
            http_status: status,
        });
    }

    serde_json::from_str(&body).map_err(|_| ReferralsError::DecodeFailed { status, body })
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
        DispenseErrorCode, DistributionSessionListOptions, ReferralInfo, ReferralListMineOptions,
        ReferralPublicListOptions, Referrals, ReferralsError, SessionErrorCode,
        SessionKeyListOptions, api_reason, dispense_error_code, normalize_base_url,
        private_key_to_address, referral_not_found_info, session_collection_url,
        session_error_code, session_keys_url,
    };
    use crate::{config, core::Core};
    use alloy_primitives::{Address, address};
    use reqwest::Client;
    use reqwest::StatusCode;
    use std::sync::Arc;

    fn test_referrals() -> Referrals {
        Referrals::with_client(
            "https://example.com/api",
            Arc::new(Core::new(config::gnosis_mainnet())),
            Client::new(),
        )
        .unwrap()
    }

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

    #[tokio::test]
    async fn resolve_auth_token_prefers_explicit_token() {
        let referrals = test_referrals().with_auth_token("configured-token");
        let token = referrals
            .resolve_auth_token(Some("explicit-token"))
            .await
            .unwrap();
        assert_eq!(token, "explicit-token");
    }

    #[tokio::test]
    async fn resolve_auth_token_uses_provider_fallback() {
        let referrals = test_referrals()
            .with_auth_token_provider(|| async { Ok("provider-token".to_string()) });
        let token = referrals.resolve_auth_token(None).await.unwrap();
        assert_eq!(token, "provider-token");
    }

    #[tokio::test]
    async fn resolve_auth_token_requires_auth_when_missing() {
        let err = test_referrals().resolve_auth_token(None).await.unwrap_err();
        assert!(matches!(err, ReferralsError::AuthRequired));
    }

    #[test]
    fn session_error_codes_match_ts_mapping() {
        assert_eq!(
            session_error_code(StatusCode::BAD_REQUEST),
            SessionErrorCode::ValidationError
        );
        assert_eq!(
            session_error_code(StatusCode::NOT_FOUND),
            SessionErrorCode::NotFound
        );
        assert_eq!(
            session_error_code(StatusCode::CONFLICT),
            SessionErrorCode::Conflict
        );
        assert_eq!(
            session_error_code(StatusCode::INTERNAL_SERVER_ERROR),
            SessionErrorCode::ServerError
        );
    }

    #[test]
    fn dispense_error_codes_match_ts_mapping() {
        assert_eq!(
            dispense_error_code(StatusCode::NOT_FOUND, "No keys available for this inviter"),
            DispenseErrorCode::PoolEmpty
        );
        assert_eq!(
            dispense_error_code(StatusCode::NOT_FOUND, "Session does not exist"),
            DispenseErrorCode::SessionNotFound
        );
        assert_eq!(
            dispense_error_code(StatusCode::GONE, "session quota exhausted"),
            DispenseErrorCode::QuotaExhausted
        );
        assert_eq!(
            dispense_error_code(StatusCode::GONE, "session expired"),
            DispenseErrorCode::SessionExpired
        );
        assert_eq!(
            dispense_error_code(StatusCode::LOCKED, "paused"),
            DispenseErrorCode::SessionPaused
        );
        assert_eq!(
            dispense_error_code(StatusCode::TOO_MANY_REQUESTS, "rate limited"),
            DispenseErrorCode::RateLimited
        );
    }

    #[test]
    fn session_collection_url_matches_ts_query_shape() {
        let url = session_collection_url(
            &normalize_base_url("https://example.com/api").unwrap(),
            address!("1111111111111111111111111111111111111111"),
            Some(&DistributionSessionListOptions {
                limit: Some(20),
                offset: Some(5),
            }),
        )
        .unwrap();

        assert_eq!(
            url.as_str(),
            "https://example.com/api/distributions/sessions?inviter=0x1111111111111111111111111111111111111111&limit=20&offset=5"
        );
    }

    #[test]
    fn session_keys_url_matches_ts_query_shape() {
        let url = session_keys_url(
            &normalize_base_url("https://example.com/api").unwrap(),
            "session-123",
            Some(&SessionKeyListOptions {
                limit: Some(10),
                offset: Some(2),
            }),
        )
        .unwrap();

        assert_eq!(
            url.as_str(),
            "https://example.com/api/distributions/sessions/session-123/keys?limit=10&offset=2"
        );
    }

    #[tokio::test]
    async fn distributions_reuse_referrals_auth_provider() {
        let token = test_referrals()
            .with_auth_token("distribution-token")
            .distributions()
            .resolve_auth_token()
            .await
            .unwrap();
        assert_eq!(token, "distribution-token");
    }
}
