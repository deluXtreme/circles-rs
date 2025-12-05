use crate::avatar::common::CommonAvatar;
use crate::cid_v0_to_digest::cid_v0_to_digest;
use crate::runner::{PreparedTransaction as RunnerTx, SubmittedTx as RunnerSubmitted};
use crate::{
    ContractRunner, Core, PreparedTransaction, Profile, SdkError, SubmittedTx, call_to_tx,
};
use alloy_primitives::{Address, U256, aliases::U96};
use alloy_sol_types::{SolCall, SolValue, sol};
use circles_abis::{HubV2, InvitationFarm, ReferralsModule};
use circles_profiles::Profiles;
use circles_rpc::CirclesRpc;
#[cfg(feature = "ws")]
use circles_rpc::events::subscription::CirclesSubscription;
#[cfg(feature = "ws")]
use circles_types::CirclesEvent;
use circles_types::{
    AdvancedTransferOptions, AvatarInfo, PathfindingResult, TokenBalanceResponse, TrustRelation,
};
use hex::encode as hex_encode;
use rand::RngCore;
use std::sync::Arc;

/// Top-level avatar enum variant: human.
pub struct HumanAvatar {
    pub address: Address,
    pub info: AvatarInfo,
    pub core: Arc<Core>,
    pub runner: Option<Arc<dyn ContractRunner>>,
    pub common: CommonAvatar,
}

/// Invitation generation result (secrets + signers + prepared txs).
#[derive(Debug, Clone)]
pub struct GeneratedInvites {
    pub secrets: Vec<String>,
    pub signers: Vec<Address>,
    pub txs: Vec<RunnerTx>,
    pub submitted: Option<Vec<RunnerSubmitted>>,
}

sol! {
    struct ReferralPayload {
        address referralsModule;
        bytes callData;
    }
}

impl HumanAvatar {
    /// Get detailed token balances (v1/v2 selectable).
    pub async fn balances(
        &self,
        as_time_circles: bool,
        use_v2: bool,
    ) -> Result<Vec<TokenBalanceResponse>, SdkError> {
        self.common.balances(as_time_circles, use_v2).await
    }

    /// Get trust relations.
    pub async fn trust_relations(&self) -> Result<Vec<TrustRelation>, SdkError> {
        self.common.trust_relations().await
    }

    /// Fetch profile (cached by CID in memory).
    pub async fn profile(&self) -> Result<Option<Profile>, SdkError> {
        self.common.profile(self.info.cid_v0.as_deref()).await
    }

    /// Update profile via profiles service and store CID through NameRegistry (requires runner).
    pub async fn update_profile(&self, profile: &Profile) -> Result<Vec<SubmittedTx>, SdkError> {
        let cid = self.common.pin_profile(profile).await?;
        let digest = cid_v0_to_digest(&cid)?;
        let call = circles_abis::NameRegistry::updateMetadataDigestCall {
            _metadataDigest: digest,
        };
        let tx = call_to_tx(self.core.config.name_registry_address, call, None);
        let sent = self.common.send(vec![tx]).await?;
        Ok(sent)
    }

    /// Trust one or more avatars via HubV2::trust (requires runner).
    pub async fn trust_add(
        &self,
        avatars: &[Address],
        expiry: u128,
    ) -> Result<Vec<SubmittedTx>, SdkError> {
        let runner = self.runner.clone().ok_or(SdkError::MissingRunner)?;
        let txs = avatars
            .iter()
            .map(|addr| HubV2::trustCall {
                _trustReceiver: *addr,
                _expiry: U96::from(expiry),
            })
            .map(|call| call_to_tx(self.core.config.v2_hub_address, call, None))
            .collect();
        Ok(runner.send_transactions(txs).await?)
    }

    /// Remove trust (sets expiry to 0). Requires runner.
    pub async fn trust_remove(&self, avatars: &[Address]) -> Result<Vec<SubmittedTx>, SdkError> {
        self.trust_add(avatars, 0).await
    }

    #[cfg(feature = "ws")]
    pub async fn subscribe_events_ws(
        &self,
        ws_url: &str,
        filter: Option<serde_json::Value>,
    ) -> Result<CirclesSubscription<CirclesEvent>, SdkError> {
        self.common.subscribe_events_ws(ws_url, filter).await
    }

    #[cfg(feature = "ws")]
    pub async fn subscribe_events_ws_with_retries(
        &self,
        ws_url: &str,
        filter: serde_json::Value,
        max_attempts: Option<usize>,
    ) -> Result<CirclesSubscription<CirclesEvent>, SdkError> {
        self.common
            .subscribe_events_ws_with_retries(ws_url, filter, max_attempts)
            .await
    }

    #[cfg(feature = "ws")]
    pub async fn subscribe_events_ws_with_catchup(
        &self,
        ws_url: &str,
        filter: serde_json::Value,
        max_attempts: Option<usize>,
        catch_up_from_block: Option<u64>,
        catch_up_filter: Option<Vec<circles_types::Filter>>,
    ) -> Result<(Vec<CirclesEvent>, CirclesSubscription<CirclesEvent>), SdkError> {
        self.common
            .subscribe_events_ws_with_catchup(
                ws_url,
                filter,
                max_attempts,
                catch_up_from_block,
                catch_up_filter,
            )
            .await
    }

    /// Plan a transfer without submitting.
    pub async fn plan_transfer(
        &self,
        to: Address,
        amount: U256,
        options: Option<AdvancedTransferOptions>,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        self.common.plan_transfer(to, amount, options).await
    }

    /// Execute a transfer using the runner (requires runner).
    pub async fn transfer(
        &self,
        to: Address,
        amount: U256,
        options: Option<AdvancedTransferOptions>,
    ) -> Result<Vec<SubmittedTx>, SdkError> {
        self.common.transfer(to, amount, options).await
    }

    pub async fn find_path(
        &self,
        to: Address,
        target_flow: U256,
        options: Option<AdvancedTransferOptions>,
    ) -> Result<PathfindingResult, SdkError> {
        self.common.find_path(to, target_flow, options).await
    }

    pub async fn max_flow_to(
        &self,
        to: Address,
        options: Option<AdvancedTransferOptions>,
    ) -> Result<PathfindingResult, SdkError> {
        self.common.max_flow_to(to, options).await
    }

    /// Generate invitation secrets/signers and return prepared transactions (claim + batch transfer).
    ///
    /// Does not submit; pair with a runner to send. Each invite funds 96 CRC.
    pub async fn generate_invites(
        &self,
        number_of_invites: u64,
    ) -> Result<GeneratedInvites, SdkError> {
        if number_of_invites == 0 {
            return Err(SdkError::InvalidRegistration(
                "number_of_invites must be greater than 0".to_string(),
            ));
        }

        // Simulate claim to retrieve ids
        let claim_call = InvitationFarm::claimInvitesCall {
            numberOfInvites: U256::from(number_of_invites),
        };
        let ids = self
            .common
            .core
            .invitation_farm()
            .claimInvites(U256::from(number_of_invites))
            .call()
            .await
            .unwrap_or_default();
        if ids.is_empty() {
            return Err(SdkError::InvalidRegistration(
                "invitation farm returned no ids".to_string(),
            ));
        }

        // Secrets/signers
        let mut secrets = Vec::with_capacity(ids.len());
        let mut signers = Vec::with_capacity(ids.len());
        for _ in &ids {
            let mut buf = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut buf);
            secrets.push(format!("0x{}", hex_encode(buf)));
            // Derive pseudo signer as lowercased hex address (no checksum)
            let signer = Address::from_slice(&buf[12..]);
            signers.push(signer);
        }

        // Referral payload
        let create_accounts = ReferralsModule::createAccountsCall {
            signers: signers.clone(),
        };
        let referrals_module = self.common.core.config.referrals_module_address;
        let payload = ReferralPayload {
            referralsModule: referrals_module,
            callData: create_accounts.abi_encode().into(),
        };
        let encoded_payload = payload.abi_encode();

        // Amounts: 96 CRC each
        let amount = U256::from(96u128) * U256::from(10).pow(U256::from(18));
        let values = vec![amount; ids.len()];

        // Build txs: claimInvites + safeBatchTransferFrom to invitation module
        let invitation_module = self
            .common
            .core
            .invitation_farm()
            .invitationModule()
            .call()
            .await
            .unwrap_or_default();

        let claim_tx = call_to_tx(
            self.common.core.config.invitation_farm_address,
            claim_call,
            None,
        );
        let batch_call = HubV2::safeBatchTransferFromCall {
            _from: self.address,
            _to: invitation_module,
            _ids: ids,
            _values: values,
            _data: encoded_payload.into(),
        };
        let batch_tx = call_to_tx(self.common.core.config.v2_hub_address, batch_call, None);

        Ok(GeneratedInvites {
            secrets,
            signers,
            txs: vec![
                RunnerTx {
                    to: claim_tx.to,
                    data: claim_tx.data,
                    value: claim_tx.value,
                },
                RunnerTx {
                    to: batch_tx.to,
                    data: batch_tx.data,
                    value: batch_tx.value,
                },
            ],
            submitted: None,
        })
    }

    /// Invitation rows (RPC helper).
    pub async fn invitations(
        &self,
    ) -> Result<Vec<circles_rpc::methods::invitation::InvitationRow>, SdkError> {
        Ok(self
            .common
            .rpc
            .invitation()
            .get_invitations(self.address)
            .await?)
    }

    /// Redeem an invitation from an inviter (requires runner).
    pub async fn redeem_invitation(&self, inviter: Address) -> Result<Vec<SubmittedTx>, SdkError> {
        let call = circles_abis::InvitationEscrow::redeemInvitationCall { inviter };
        let tx = call_to_tx(
            self.common.core.config.invitation_escrow_address,
            call,
            None,
        );
        self.common.send(vec![tx]).await
    }

    /// Revoke a specific invitation (requires runner).
    pub async fn revoke_invitation(&self, invitee: Address) -> Result<Vec<SubmittedTx>, SdkError> {
        let call = circles_abis::InvitationEscrow::revokeInvitationCall { invitee };
        let tx = call_to_tx(
            self.common.core.config.invitation_escrow_address,
            call,
            None,
        );
        self.common.send(vec![tx]).await
    }

    /// Revoke all invitations sent by this avatar (requires runner).
    pub async fn revoke_all_invitations(&self) -> Result<Vec<SubmittedTx>, SdkError> {
        let call = circles_abis::InvitationEscrow::revokeAllInvitationsCall {};
        let tx = call_to_tx(
            self.common.core.config.invitation_escrow_address,
            call,
            None,
        );
        self.common.send(vec![tx]).await
    }

    pub fn new(
        address: Address,
        info: AvatarInfo,
        core: Arc<Core>,
        profiles: Profiles,
        rpc: Arc<CirclesRpc>,
        runner: Option<Arc<dyn ContractRunner>>,
    ) -> Self {
        let common = CommonAvatar::new(address, core.clone(), profiles, rpc, runner.clone());
        Self {
            address,
            info,
            core,
            runner,
            common,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Bytes, address};

    #[test]
    fn referral_payload_encodes() {
        let signers = vec![address!("1000000000000000000000000000000000000001")];
        let create_accounts = ReferralsModule::createAccountsCall {
            signers: signers.clone(),
        };
        let payload = ReferralPayload {
            referralsModule: address!("2000000000000000000000000000000000000002"),
            callData: create_accounts.abi_encode().into(),
        };
        let bytes = payload.abi_encode();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn batch_tx_targets_hub() {
        let ids = vec![U256::from(1), U256::from(2)];
        let amount = U256::from(96u128) * U256::from(10).pow(U256::from(18));
        let values = vec![amount; ids.len()];
        let batch_call = HubV2::safeBatchTransferFromCall {
            _from: address!("aaaa000000000000000000000000000000000000"),
            _to: address!("bbbb000000000000000000000000000000000000"),
            _ids: ids,
            _values: values,
            _data: Bytes::default(),
        };
        let batch_tx = call_to_tx(
            address!("cccc000000000000000000000000000000000000"),
            batch_call,
            None,
        );
        assert_eq!(
            batch_tx.to,
            address!("cccc000000000000000000000000000000000000")
        );
    }
}
