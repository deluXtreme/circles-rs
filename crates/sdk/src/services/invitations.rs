use crate::avatar::human::{HumanAvatar, ProxyInviter, ReferralCodePlan};
use crate::services::invite_farm::GeneratedReferral;
use crate::services::referrals::{
    ReferralPreviewList, ReferralPublicListOptions, Referrals, generate_private_key,
    private_key_to_address,
};
use crate::{ContractRunner, Core, PreparedTransaction, SdkError};
use alloy_primitives::{Address, Bytes};
use alloy_sol_types::{SolCall, SolValue, sol};
use circles_abis::ReferralsModule;
use circles_profiles::Profiles;
use circles_rpc::CirclesRpc;
use circles_types::PathfindingResult;
use std::sync::Arc;

sol! {
    struct ReferralPayload {
        address referralsModule;
        bytes callData;
    }
}

/// Dedicated invitations facade mirroring the TypeScript SDK service surface.
#[derive(Clone)]
pub struct Invitations {
    core: Arc<Core>,
    profiles: Profiles,
    rpc: Arc<CirclesRpc>,
    runner: Option<Arc<dyn ContractRunner>>,
    referrals: Option<Referrals>,
}

impl Invitations {
    pub fn new(
        core: Arc<Core>,
        profiles: Profiles,
        rpc: Arc<CirclesRpc>,
        runner: Option<Arc<dyn ContractRunner>>,
        referrals: Option<Referrals>,
    ) -> Self {
        Self {
            core,
            profiles,
            rpc,
            runner,
            referrals,
        }
    }

    /// Check whether an inviter Safe needs module-enable or trust setup transactions.
    pub async fn ensure_inviter_setup(
        &self,
        inviter: Address,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        self.human_avatar(inviter)
            .await?
            .ensure_inviter_setup()
            .await
    }

    /// Store referral data in the optional referrals backend.
    pub async fn save_referral_data(
        &self,
        inviter: Address,
        private_key: &str,
    ) -> Result<(), SdkError> {
        Ok(self.referrals()?.store(private_key, inviter).await?)
    }

    /// List public referral previews for an inviter.
    pub async fn list_referrals(
        &self,
        inviter: Address,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<ReferralPreviewList, SdkError> {
        Ok(self
            .referrals()?
            .list_public(
                inviter,
                Some(ReferralPublicListOptions {
                    limit,
                    offset,
                    in_session: None,
                }),
            )
            .await?)
    }

    /// Find a proxy-inviter-backed invitation path for a specific inviter.
    pub async fn find_invite_path(
        &self,
        inviter: Address,
        proxy_inviter_address: Option<Address>,
    ) -> Result<PathfindingResult, SdkError> {
        self.human_avatar(inviter)
            .await?
            .find_invite_path(proxy_inviter_address)
            .await
    }

    /// Find a farm-backed fallback invitation path for a specific inviter.
    pub async fn find_farm_invite_path(
        &self,
        inviter: Address,
    ) -> Result<PathfindingResult, SdkError> {
        self.human_avatar(inviter)
            .await?
            .find_farm_invite_path()
            .await
    }

    /// Get real inviters who can currently route invitation flow for an inviter.
    pub async fn get_real_inviters(&self, inviter: Address) -> Result<Vec<ProxyInviter>, SdkError> {
        self.human_avatar(inviter).await?.proxy_inviters().await
    }

    /// Plan a direct invite for an existing Safe wallet without executing it.
    pub async fn generate_invite(
        &self,
        inviter: Address,
        invitee: Address,
    ) -> Result<Vec<PreparedTransaction>, SdkError> {
        self.human_avatar(inviter).await?.plan_invite(invitee).await
    }

    /// Plan a single referral code for a new invitee without executing it.
    pub async fn generate_referral(&self, inviter: Address) -> Result<ReferralCodePlan, SdkError> {
        self.human_avatar(inviter).await?.plan_referral_code().await
    }

    /// Build the TS-style invitation payload for direct invite or referral-account creation.
    pub fn generate_invite_data(
        &self,
        addresses: &[Address],
        use_safe_creation: bool,
    ) -> Result<Bytes, SdkError> {
        if addresses.is_empty() {
            return Err(SdkError::OperationFailed(
                "no addresses provided for invitation payload generation".to_string(),
            ));
        }

        if !use_safe_creation {
            return Ok(if addresses.len() == 1 {
                Bytes::from(addresses[0].abi_encode())
            } else {
                Bytes::from(addresses.to_vec().abi_encode())
            });
        }

        let call_data: Bytes = if addresses.len() == 1 {
            ReferralsModule::createAccountCall {
                signer: addresses[0],
            }
            .abi_encode()
            .into()
        } else {
            ReferralsModule::createAccountsCall {
                signers: addresses.to_vec(),
            }
            .abi_encode()
            .into()
        };

        Ok(Bytes::from(
            ReferralPayload {
                referralsModule: self.core.config.referrals_module_address,
                callData: call_data,
            }
            .abi_encode(),
        ))
    }

    /// Compute the deterministic referrals-module Safe address for a signer.
    pub async fn compute_address(&self, signer: Address) -> Result<Address, SdkError> {
        self.core
            .referrals_module()
            .computeAddress(signer)
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
    }

    /// Generate referral secrets plus their derived signer addresses.
    pub fn generate_secrets(&self, count: usize) -> Result<Vec<GeneratedReferral>, SdkError> {
        let mut generated = Vec::with_capacity(count);
        for _ in 0..count {
            let secret = generate_private_key();
            let signer = private_key_to_address(&secret)?;
            generated.push(GeneratedReferral { secret, signer });
        }
        Ok(generated)
    }

    async fn human_avatar(&self, inviter: Address) -> Result<HumanAvatar, SdkError> {
        let info = self.rpc.avatar().get_avatar_info(inviter).await?;
        Ok(HumanAvatar::new(
            inviter,
            info,
            self.core.clone(),
            self.profiles.clone(),
            self.rpc.clone(),
            self.runner.clone(),
        ))
    }

    fn referrals(&self) -> Result<&Referrals, SdkError> {
        self.referrals.as_ref().ok_or_else(|| {
            SdkError::OperationFailed(
                "Referrals service not configured. Set referrals_service_url in CirclesConfig."
                    .to_string(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Invitations, ReferralPayload};
    use crate::services::invite_farm::GeneratedReferral;
    use crate::{SdkError, config, core::Core};
    use alloy_primitives::{Address, Bytes, address};
    use alloy_sol_types::{SolCall, SolValue};
    use circles_abis::ReferralsModule;
    use circles_profiles::Profiles;
    use circles_rpc::CirclesRpc;
    use std::sync::Arc;

    fn test_service() -> Invitations {
        let cfg = config::gnosis_mainnet();
        let core = Arc::new(Core::new(cfg.clone()));
        let profiles = Profiles::new(cfg.effective_profile_service_url()).expect("profiles");
        let rpc = Arc::new(CirclesRpc::try_from_http(&cfg.circles_rpc_url).expect("rpc"));
        Invitations::new(core, profiles, rpc, None, None)
    }

    #[test]
    fn generate_invite_data_matches_single_direct_address_shape() {
        let service = test_service();
        let invitee = address!("1000000000000000000000000000000000000001");

        let encoded = service
            .generate_invite_data(&[invitee], false)
            .expect("direct payload");

        assert_eq!(encoded, Bytes::from(invitee.abi_encode()));
    }

    #[test]
    fn generate_invite_data_matches_safe_creation_batch_payload() {
        let service = test_service();
        let signers = vec![
            address!("1000000000000000000000000000000000000001"),
            address!("2000000000000000000000000000000000000002"),
        ];

        let encoded = service
            .generate_invite_data(&signers, true)
            .expect("safe-creation payload");

        let expected = Bytes::from(
            ReferralPayload {
                referralsModule: config::gnosis_mainnet().referrals_module_address,
                callData: ReferralsModule::createAccountsCall {
                    signers: signers.clone(),
                }
                .abi_encode()
                .into(),
            }
            .abi_encode(),
        );

        assert_eq!(encoded, expected);
    }

    #[test]
    fn generate_secrets_preserves_count_and_derives_signers() {
        let service = test_service();
        let generated = service.generate_secrets(3).expect("generated");

        assert_eq!(generated.len(), 3);
        assert!(
            generated
                .iter()
                .all(|GeneratedReferral { signer, .. }| { *signer != Address::ZERO })
        );
    }

    #[test]
    fn generate_invite_data_rejects_empty_addresses() {
        let service = test_service();
        let err = service
            .generate_invite_data(&[], true)
            .expect_err("empty addresses should fail");

        match err {
            SdkError::OperationFailed(message) => {
                assert!(message.contains("no addresses provided"))
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn list_referrals_requires_referrals_backend() {
        let service = test_service();
        let inviter = address!("3000000000000000000000000000000000000003");

        let err = service
            .list_referrals(inviter, None, None)
            .await
            .expect_err("missing referrals backend");

        match err {
            SdkError::OperationFailed(message) => {
                assert!(message.contains("Referrals service not configured"))
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
