//! Client for the Circles profile service (pin + fetch profile metadata).
//! Mirrors the minimal behavior of the TypeScript `@profiles` package.

pub use circles_types::{GroupProfile, Profile};
use reqwest::{Client, StatusCode, Url};
use thiserror::Error;

/// Errors that can occur when interacting with the profile service.
#[derive(Debug, Error)]
pub enum ProfilesError {
    /// The provided base URL was invalid.
    #[error("invalid profile service url `{url}`: {source}")]
    InvalidUrl {
        url: String,
        #[source]
        source: url::ParseError,
    },
    /// The provided URL cannot be treated as a base for relative paths.
    #[error("profile service url cannot be a base: {url}")]
    CannotBeABase { url: String },
    /// HTTP-layer error when sending or receiving requests.
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),
    /// The service returned a non-success status during profile creation.
    #[error("profile creation failed (status {status}): {body}")]
    CreateFailed { status: StatusCode, body: String },
    /// The service responded with an unexpected payload.
    #[error("unexpected response format (status {status}): {body}")]
    DecodeFailed { status: StatusCode, body: String },
}

/// Thin wrapper over the Circles profile service.
#[derive(Debug, Clone)]
pub struct Profiles {
    base_url: Url,
    client: Client,
}

impl Profiles {
    /// Build a client using the default Reqwest client.
    pub fn new(profile_service_url: impl AsRef<str>) -> Result<Self, ProfilesError> {
        Self::with_client(profile_service_url, Client::new())
    }

    /// Build a client using a provided Reqwest client (useful for custom middleware or mocks).
    pub fn with_client(
        profile_service_url: impl AsRef<str>,
        client: Client,
    ) -> Result<Self, ProfilesError> {
        let base_url = normalize_base_url(profile_service_url.as_ref())?;
        Ok(Self { base_url, client })
    }

    /// Create and pin a profile, returning its CID.
    pub async fn create(&self, profile: &Profile) -> Result<String, ProfilesError> {
        let url = endpoint(&self.base_url, "pin")?;
        let response = self.client.post(url).json(profile).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ProfilesError::CreateFailed { status, body });
        }

        let cid = serde_json::from_str::<PinResponse>(&body)
            .map_err(|_| ProfilesError::DecodeFailed { status, body })?;
        Ok(cid.cid)
    }

    /// Retrieve a profile by CID. Returns `Ok(None)` if the service responds with a non-success
    /// status or if the body cannot be parsed.
    pub async fn get(&self, cid: &str) -> Result<Option<Profile>, ProfilesError> {
        let mut url = endpoint(&self.base_url, "get")?;
        url.query_pairs_mut().append_pair("cid", cid);

        let response = self.client.get(url).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            tracing::warn!(
                %status,
                cid,
                body = body.as_str(),
                "failed to retrieve profile"
            );
            return Ok(None);
        }

        match serde_json::from_str(&body) {
            Ok(profile) => Ok(Some(profile)),
            Err(err) => {
                tracing::warn!(
                    %status,
                    cid,
                    body = body.as_str(),
                    error = %err,
                    "failed to parse profile response"
                );
                Ok(None)
            }
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct PinResponse {
    cid: String,
}

fn endpoint(base: &Url, path: &str) -> Result<Url, ProfilesError> {
    base.join(path).map_err(|source| ProfilesError::InvalidUrl {
        url: format!("{base}{path}"),
        source,
    })
}

fn normalize_base_url(raw: &str) -> Result<Url, ProfilesError> {
    let mut url = Url::parse(raw).map_err(|source| ProfilesError::InvalidUrl {
        url: raw.to_owned(),
        source,
    })?;

    if !url.path().ends_with('/') {
        url.path_segments_mut()
            .map_err(|_| ProfilesError::CannotBeABase {
                url: raw.to_owned(),
            })?
            .push("");
    }

    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::normalize_base_url;

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
}
