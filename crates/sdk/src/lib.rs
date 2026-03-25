//! Circles SDK orchestrating RPC, profile service access, pathfinding, transfers,
//! and optional contract execution.
//!
//! This crate mirrors the high-level TypeScript SDK shape while keeping the Rust
//! implementation read-first: most reads work with `Sdk::new(config, None)`, and
//! write paths are gated behind a [`ContractRunner`].
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use alloy_primitives::address;
//! use circles_sdk::{Sdk, config};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let sdk = Sdk::new(config::gnosis_mainnet(), None)?;
//! let avatar = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
//! let info = sdk.avatar_info(avatar).await?;
//! println!("avatar type: {:?}", info.avatar_type);
//!
//! let typed = sdk.get_avatar(avatar).await?;
//! match typed {
//!     circles_sdk::Avatar::Human(human) => {
//!         let balances = human.balances(false, true).await?;
//!         println!("balances: {}", balances.len());
//!     }
//!     circles_sdk::Avatar::Organisation(_) | circles_sdk::Avatar::Group(_) => {}
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Usage Model
//!
//! - [`Sdk`] wires together RPC, profile lookups, pathfinding, transfers, and contract bindings.
//! - [`Avatar`] gives you a typed wrapper after runtime avatar detection.
//! - [`ContractRunner`] is only required for write paths such as registrations, trust changes,
//!   and transfer submission.
//! - The optional `ws` feature enables WebSocket subscriptions with retry/backoff and HTTP catch-up helpers.
//!
//! ## Recommended Entry Points
//!
//! - [`config::gnosis_mainnet`] for the shared mainnet configuration.
//! - [`Sdk::avatar_info`] for a fast read-only probe.
//! - [`Sdk::get_avatar`] when you want a typed avatar wrapper.
//! - [`HumanAvatar::plan_transfer`], [`OrganisationAvatar::plan_transfer`], and
//!   [`BaseGroupAvatar::plan_transfer`] for transaction planning before submission.
//!
//! ## Validation
//!
//! - Unit tests: `cargo test -p circles-sdk`
//! - WS helpers: `cargo test -p circles-sdk --features ws`
//! - Live checks (ignored by default): `RUN_LIVE=1 LIVE_AVATAR=0x... cargo test -p circles-sdk -- --ignored`

mod avatar;
mod cid_v0_to_digest;
pub mod config;
mod core;
mod runner;
mod services;
#[cfg(feature = "ws")]
pub mod ws;
pub use services::registration;

#[cfg(feature = "ws")]
use alloy_json_rpc::RpcSend;
use alloy_primitives::Address;
pub use avatar::{BaseGroupAvatar, HumanAvatar, OrganisationAvatar};
use circles_profiles::{Profile, Profiles};
use circles_rpc::CirclesRpc;
#[cfg(feature = "ws")]
use circles_rpc::events::subscription::CirclesSubscription;
#[cfg(feature = "ws")]
use circles_types::CirclesEvent;
use circles_types::{AvatarInfo, AvatarType, CirclesConfig, TokenBalanceResponse, TrustRelation};
use core::Core;
pub use runner::{ContractRunner, PreparedTransaction, RunnerError, SubmittedTx, call_to_tx};
#[cfg(feature = "ws")]
use serde_json::to_value;
use std::sync::Arc;
use thiserror::Error;

/// Generic registration outcome carrying submitted transactions and an optional avatar.
///
/// Registration helpers may return prepared txs without sending if no runner is provided.
pub struct RegistrationResult<T> {
    /// Best-effort typed avatar returned after registration succeeds.
    pub avatar: Option<T>,
    /// Submitted transactions returned by the runner.
    pub txs: Vec<SubmittedTx>,
}

/// High-level SDK errors.
#[derive(Debug, Error)]
pub enum SdkError {
    #[error("circles rpc error: {0}")]
    Rpc(#[from] circles_rpc::CirclesRpcError),
    #[error("profiles error: {0}")]
    Profiles(#[from] circles_profiles::ProfilesError),
    #[error("transfers error: {0}")]
    Transfers(#[from] circles_transfers::TransferError),
    #[error("runner error: {0}")]
    Runner(#[from] RunnerError),
    #[error("cid error: {0}")]
    Cid(#[from] cid_v0_to_digest::CidError),
    #[error("contract runner is required for this operation")]
    MissingRunner,
    #[error("sender address is required for this operation")]
    MissingSender,
    #[error("avatar not found for address {0:?}")]
    AvatarNotFound(Address),
    #[error("invalid registration input: {0}")]
    InvalidRegistration(String),
    #[error("websocket subscription failed after {attempts} attempts: {reason}")]
    WsSubscribeFailed { attempts: usize, reason: String },
}

/// Top-level SDK orchestrator.
///
/// Construct this once per config/runner pair and reuse it across read and write flows.
pub struct Sdk {
    pub(crate) config: CirclesConfig,
    pub(crate) rpc: Arc<CirclesRpc>,
    pub(crate) profiles: Profiles,
    pub(crate) core: Arc<Core>,
    pub(crate) runner: Option<Arc<dyn ContractRunner>>,
    pub(crate) sender_address: Option<Address>,
}

impl Sdk {
    /// Create a new SDK instance. Provide a runner for write operations; omit for read-only.
    pub fn new(
        config: CirclesConfig,
        runner: Option<Arc<dyn ContractRunner>>,
    ) -> Result<Self, SdkError> {
        let sender_address = runner.as_ref().map(|r| r.sender_address());
        let core = Arc::new(Core::new(config.clone()));
        let rpc = Arc::new(CirclesRpc::try_from_http(&config.circles_rpc_url)?);
        let profiles = Profiles::new(config.profile_service_url.clone())?;
        Ok(Self {
            rpc,
            profiles,
            config,
            core,
            runner,
            sender_address,
        })
    }

    /// Access the underlying RPC client.
    pub fn rpc(&self) -> &CirclesRpc {
        self.rpc.as_ref()
    }

    /// Access the loaded configuration.
    pub fn config(&self) -> &CirclesConfig {
        &self.config
    }

    /// Access core contract bundle.
    pub fn core(&self) -> &Arc<Core> {
        &self.core
    }

    /// Access the profiles client.
    pub fn profiles(&self) -> &Profiles {
        &self.profiles
    }

    /// Optional runner.
    pub fn runner(&self) -> Option<&Arc<dyn ContractRunner>> {
        self.runner.as_ref()
    }

    /// Sender address derived from the runner.
    pub fn sender_address(&self) -> Option<Address> {
        self.sender_address
    }

    /// Create and pin a profile via the profile service.
    ///
    /// This only talks to the profile service and does not submit any on-chain transaction.
    pub async fn create_profile(&self, profile: &Profile) -> Result<String, SdkError> {
        Ok(self.profiles.create(profile).await?)
    }

    /// Fetch a profile by CID (returns `Ok(None)` if missing or unparsable).
    pub async fn get_profile(&self, cid: &str) -> Result<Option<Profile>, SdkError> {
        Ok(self.profiles.get(cid).await?)
    }

    /// Read avatar metadata directly from the RPC service.
    pub async fn data_avatar(&self, avatar: Address) -> Result<AvatarInfo, SdkError> {
        Ok(self.rpc.avatar().get_avatar_info(avatar).await?)
    }

    /// Read trust relations for an avatar directly from the RPC service.
    pub async fn data_trust(&self, avatar: Address) -> Result<Vec<TrustRelation>, SdkError> {
        Ok(self.rpc.trust().get_trust_relations(avatar).await?)
    }

    /// Read token balances for an avatar directly from the RPC service.
    ///
    /// Set `as_time_circles` to request balances in time-Circles units and `use_v2`
    /// to scope the query to v2 balances.
    pub async fn data_balances(
        &self,
        avatar: Address,
        as_time_circles: bool,
        use_v2: bool,
    ) -> Result<Vec<TokenBalanceResponse>, SdkError> {
        Ok(self
            .rpc
            .token()
            .get_token_balances(avatar, as_time_circles, use_v2)
            .await?)
    }

    /// Convenience accessor for avatar info (read-only).
    pub async fn avatar_info(&self, avatar: Address) -> Result<AvatarInfo, SdkError> {
        Ok(self.rpc.avatar().get_avatar_info(avatar).await?)
    }

    /// Subscribe to Circles events over WebSocket with a custom JSON-RPC filter payload.
    #[cfg(feature = "ws")]
    pub async fn subscribe_events_ws<F>(
        &self,
        ws_url: &str,
        filter: F,
    ) -> Result<CirclesSubscription<CirclesEvent>, SdkError>
    where
        F: RpcSend + 'static,
    {
        let val = to_value(&filter).map_err(|e| SdkError::WsSubscribeFailed {
            attempts: 0,
            reason: e.to_string(),
        })?;
        self.subscribe_events_ws_with_retries(ws_url, val, None)
            .await
    }

    /// Subscribe with retry/backoff on WebSocket connection or subscription failure.
    #[cfg(feature = "ws")]
    pub async fn subscribe_events_ws_with_retries(
        &self,
        ws_url: &str,
        filter: serde_json::Value,
        max_attempts: Option<usize>,
    ) -> Result<CirclesSubscription<CirclesEvent>, SdkError> {
        ws::subscribe_with_retries(ws_url, filter, max_attempts).await
    }

    /// Subscribe with retry/backoff and optionally fetch historical events first over HTTP.
    #[cfg(feature = "ws")]
    pub async fn subscribe_events_ws_with_catchup(
        &self,
        ws_url: &str,
        filter: serde_json::Value,
        max_attempts: Option<usize>,
        catch_up_from_block: Option<u64>,
        catch_up_filter: Option<Vec<circles_types::Filter>>,
    ) -> Result<(Vec<CirclesEvent>, CirclesSubscription<CirclesEvent>), SdkError> {
        ws::subscribe_with_catchup(
            self.rpc.as_ref(),
            ws_url,
            filter,
            max_attempts,
            catch_up_from_block,
            catch_up_filter,
            None,
        )
        .await
    }

    /// Fetch avatar info and return the matching typed avatar wrapper.
    ///
    /// Unknown or personal avatar types are treated as [`Avatar::Human`] to match the
    /// current SDK behavior.
    pub async fn get_avatar(&self, avatar: Address) -> Result<Avatar, SdkError> {
        let info = self.rpc.avatar().get_avatar_info(avatar).await?;
        Ok(match info.avatar_type {
            AvatarType::CrcV2RegisterGroup => Avatar::Group(BaseGroupAvatar::new(
                avatar,
                info,
                self.core.clone(),
                self.profiles.clone(),
                self.rpc.clone(),
                self.runner.clone(),
            )),
            AvatarType::CrcV2RegisterOrganization => Avatar::Organisation(OrganisationAvatar::new(
                avatar,
                info,
                self.core.clone(),
                self.profiles.clone(),
                self.rpc.clone(),
                self.runner.clone(),
            )),
            _ => Avatar::Human(HumanAvatar::new(
                avatar,
                info,
                self.core.clone(),
                self.profiles.clone(),
                self.rpc.clone(),
                self.runner.clone(),
            )),
        })
    }

    /// Register a human avatar (profile is pinned before submission). Requires a runner.
    pub async fn register_human(
        &self,
        inviter: Address,
        profile: &Profile,
    ) -> Result<RegistrationResult<HumanAvatar>, SdkError> {
        registration::register_human(self, inviter, profile).await
    }

    /// Register an organisation avatar. Requires a runner.
    pub async fn register_organisation(
        &self,
        name: &str,
        profile: &Profile,
    ) -> Result<RegistrationResult<OrganisationAvatar>, SdkError> {
        registration::register_organisation(self, name, profile).await
    }

    /// Register a base group via the factory. Returns submitted txs and best-effort avatar.
    #[allow(clippy::too_many_arguments)]
    pub async fn register_group(
        &self,
        owner: Address,
        service: Address,
        fee_collection: Address,
        initial_conditions: &[Address],
        name: &str,
        symbol: &str,
        profile: &Profile,
    ) -> Result<RegistrationResult<BaseGroupAvatar>, SdkError> {
        registration::register_group(
            self,
            owner,
            service,
            fee_collection,
            initial_conditions,
            name,
            symbol,
            profile,
        )
        .await
    }
}

/// Top-level avatar enum (human, organisation, group).
pub enum Avatar {
    /// Human or personal avatar wrapper.
    Human(HumanAvatar),
    /// Organisation avatar wrapper.
    Organisation(OrganisationAvatar),
    /// Base group avatar wrapper.
    Group(BaseGroupAvatar),
}
