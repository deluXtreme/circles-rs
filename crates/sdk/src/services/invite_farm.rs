use crate::services::referrals::{
    ReferralPreviewList, ReferralPublicListOptions, Referrals, generate_private_key,
    private_key_to_address,
};
use crate::{Core, PreparedTransaction, SdkError, call_to_tx};
use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::{SolCall, SolValue, sol};
use circles_abis::{HubV2, InvitationFarm, ReferralsModule};
use std::sync::Arc;

/// One generated referral secret plus its derived signer address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedReferral {
    pub secret: String,
    pub signer: Address,
}

/// Result of planning TS-style batch referral generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateReferralsResult {
    pub referrals: Vec<GeneratedReferral>,
    pub transactions: Vec<PreparedTransaction>,
}

/// Result of planning TS-style batch invites for existing accounts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateInvitesResult {
    pub invitees: Vec<Address>,
    pub transactions: Vec<PreparedTransaction>,
}

/// Dedicated invitation-farm facade mirroring the TypeScript SDK surface.
#[derive(Clone)]
pub struct InviteFarm {
    core: Arc<Core>,
    referrals: Option<Referrals>,
}

sol! {
    struct ReferralPayload {
        address referralsModule;
        bytes callData;
    }
}

impl InviteFarm {
    pub fn new(core: Arc<Core>, referrals: Option<Referrals>) -> Self {
        Self { core, referrals }
    }

    /// Remaining invitation quota for a specific inviter.
    pub async fn quota(&self, inviter: Address) -> Result<U256, SdkError> {
        self.core
            .invitation_farm()
            .inviterQuota(inviter)
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
    }

    /// Invitation fee currently configured on the invitation farm.
    pub async fn invitation_fee(&self) -> Result<U256, SdkError> {
        self.core
            .invitation_farm()
            .INVITATION_FEE()
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
    }

    /// Invitation module address currently configured on the invitation farm.
    pub async fn invitation_module(&self) -> Result<Address, SdkError> {
        self.core
            .invitation_farm()
            .invitationModule()
            .call()
            .await
            .map_err(|e| SdkError::Contract(e.to_string()))
    }

    /// Plan batch referrals for new accounts via the invitation farm.
    pub async fn generate_referrals(
        &self,
        inviter: Address,
        count: u64,
    ) -> Result<GenerateReferralsResult, SdkError> {
        if count == 0 {
            return Err(SdkError::OperationFailed(
                "count must be greater than 0".to_string(),
            ));
        }

        let ids = self.simulate_claim(inviter, count).await?;
        if ids.is_empty() {
            return Err(SdkError::OperationFailed(
                "invitation farm returned no ids".to_string(),
            ));
        }

        let referrals = generate_referral_secrets(ids.len())?;
        let signers = referrals
            .iter()
            .map(|referral| referral.signer)
            .collect::<Vec<_>>();
        let invitation_module = self.invitation_module().await?;

        Ok(GenerateReferralsResult {
            referrals,
            transactions: vec![
                build_claim_tx(self.core.config.invitation_farm_address, count),
                build_referral_transfer_tx(
                    self.core.config.v2_hub_address,
                    inviter,
                    invitation_module,
                    ids,
                    &signers,
                    self.core.config.referrals_module_address,
                ),
            ],
        })
    }

    /// Plan batch invites for existing accounts via the invitation farm.
    pub async fn generate_invites(
        &self,
        inviter: Address,
        invitees: &[Address],
    ) -> Result<GenerateInvitesResult, SdkError> {
        if invitees.is_empty() {
            return Err(SdkError::OperationFailed(
                "at least one invitee address must be provided".to_string(),
            ));
        }

        let ids = self.simulate_claim(inviter, invitees.len() as u64).await?;
        if ids.is_empty() {
            return Err(SdkError::OperationFailed(
                "invitation farm returned no ids".to_string(),
            ));
        }

        let invitation_module = self.invitation_module().await?;

        Ok(GenerateInvitesResult {
            invitees: invitees.to_vec(),
            transactions: vec![
                build_claim_tx(
                    self.core.config.invitation_farm_address,
                    invitees.len() as u64,
                ),
                build_invite_transfer_tx(
                    self.core.config.v2_hub_address,
                    inviter,
                    invitation_module,
                    ids,
                    invitees,
                ),
            ],
        })
    }

    /// List public referral previews for an inviter via the optional referrals backend.
    pub async fn list_referrals(
        &self,
        inviter: Address,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<ReferralPreviewList, SdkError> {
        let referrals = self.referrals.as_ref().ok_or_else(|| {
            SdkError::OperationFailed(
                "Referrals service not configured. Set referrals_service_url in CirclesConfig."
                    .to_string(),
            )
        })?;

        Ok(referrals
            .list_public(
                inviter,
                Some(ReferralPublicListOptions {
                    limit: Some(limit.unwrap_or(10)),
                    offset: Some(offset.unwrap_or(0)),
                    in_session: None,
                }),
            )
            .await?)
    }

    async fn simulate_claim(&self, inviter: Address, count: u64) -> Result<Vec<U256>, SdkError> {
        if count == 1 {
            let id = self
                .core
                .invitation_farm()
                .claimInvite()
                .from(inviter)
                .call()
                .await
                .map_err(|e| SdkError::Contract(e.to_string()))?;
            Ok(vec![id])
        } else {
            self.core
                .invitation_farm()
                .claimInvites(U256::from(count))
                .from(inviter)
                .call()
                .await
                .map_err(|e| SdkError::Contract(e.to_string()))
        }
    }
}

fn invitation_fee_amount() -> U256 {
    U256::from(96u128) * U256::from(10).pow(U256::from(18))
}

fn generate_referral_secrets(count: usize) -> Result<Vec<GeneratedReferral>, SdkError> {
    let mut referrals = Vec::with_capacity(count);

    for _ in 0..count {
        let secret = generate_private_key();
        let signer = private_key_to_address(&secret)?;
        referrals.push(GeneratedReferral { secret, signer });
    }

    Ok(referrals)
}

fn build_claim_tx(invitation_farm: Address, count: u64) -> PreparedTransaction {
    if count == 1 {
        call_to_tx(invitation_farm, InvitationFarm::claimInviteCall {}, None)
    } else {
        call_to_tx(
            invitation_farm,
            InvitationFarm::claimInvitesCall {
                numberOfInvites: U256::from(count),
            },
            None,
        )
    }
}

fn encode_referral_data(signers: &[Address], referrals_module: Address) -> Bytes {
    let call_data = if signers.len() == 1 {
        ReferralsModule::createAccountCall { signer: signers[0] }
            .abi_encode()
            .into()
    } else {
        ReferralsModule::createAccountsCall {
            signers: signers.to_vec(),
        }
        .abi_encode()
        .into()
    };

    Bytes::from(
        ReferralPayload {
            referralsModule: referrals_module,
            callData: call_data,
        }
        .abi_encode(),
    )
}

fn encode_invitees_data(invitees: &[Address]) -> Bytes {
    if invitees.len() == 1 {
        Bytes::from(invitees[0].abi_encode())
    } else {
        Bytes::from(invitees.to_vec().abi_encode())
    }
}

fn build_referral_transfer_tx(
    hub: Address,
    from: Address,
    invitation_module: Address,
    ids: Vec<U256>,
    signers: &[Address],
    referrals_module: Address,
) -> PreparedTransaction {
    let data = encode_referral_data(signers, referrals_module);

    if ids.len() == 1 {
        call_to_tx(
            hub,
            HubV2::safeTransferFromCall {
                _from: from,
                _to: invitation_module,
                _id: ids[0],
                _value: invitation_fee_amount(),
                _data: data,
            },
            None,
        )
    } else {
        let values = vec![invitation_fee_amount(); ids.len()];
        call_to_tx(
            hub,
            HubV2::safeBatchTransferFromCall {
                _from: from,
                _to: invitation_module,
                _ids: ids,
                _values: values,
                _data: data,
            },
            None,
        )
    }
}

fn build_invite_transfer_tx(
    hub: Address,
    from: Address,
    invitation_module: Address,
    ids: Vec<U256>,
    invitees: &[Address],
) -> PreparedTransaction {
    let data = encode_invitees_data(invitees);

    if ids.len() == 1 {
        call_to_tx(
            hub,
            HubV2::safeTransferFromCall {
                _from: from,
                _to: invitation_module,
                _id: ids[0],
                _value: invitation_fee_amount(),
                _data: data,
            },
            None,
        )
    } else {
        let values = vec![invitation_fee_amount(); ids.len()];
        call_to_tx(
            hub,
            HubV2::safeBatchTransferFromCall {
                _from: from,
                _to: invitation_module,
                _ids: ids,
                _values: values,
                _data: data,
            },
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{InviteFarm, build_claim_tx, encode_invitees_data, generate_referral_secrets};
    use crate::{Core, SdkError, config};
    use alloy_primitives::{Address, U256, address};
    use alloy_sol_types::{SolCall, SolValue};
    use circles_abis::InvitationFarm;
    use std::sync::Arc;

    #[test]
    fn encode_invitees_data_matches_single_address_shape() {
        let invitee = address!("1000000000000000000000000000000000000001");
        assert_eq!(
            encode_invitees_data(&[invitee]),
            alloy_primitives::Bytes::from(invitee.abi_encode())
        );
    }

    #[test]
    fn encode_invitees_data_matches_address_array_shape() {
        let invitees = vec![
            address!("1000000000000000000000000000000000000001"),
            address!("2000000000000000000000000000000000000002"),
        ];
        assert_eq!(
            encode_invitees_data(&invitees),
            alloy_primitives::Bytes::from(invitees.clone().abi_encode())
        );
    }

    #[test]
    fn build_claim_tx_uses_single_and_batch_selectors() {
        let farm = address!("3000000000000000000000000000000000000003");

        let single = build_claim_tx(farm, 1);
        assert_eq!(single.to, farm);
        assert_eq!(
            &single.data[..4],
            &InvitationFarm::claimInviteCall {}.abi_encode()[..4]
        );

        let batch = build_claim_tx(farm, 3);
        assert_eq!(batch.to, farm);
        assert_eq!(
            &batch.data[..4],
            &InvitationFarm::claimInvitesCall {
                numberOfInvites: U256::from(3u64),
            }
            .abi_encode()[..4]
        );
    }

    #[test]
    fn generate_referral_secrets_preserve_count() {
        let referrals = generate_referral_secrets(3).expect("generated referrals");
        assert_eq!(referrals.len(), 3);
        assert!(
            referrals
                .iter()
                .all(|referral| referral.signer != Address::ZERO)
        );
    }

    #[test]
    fn list_referrals_requires_referrals_backend() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let service = InviteFarm::new(Arc::new(Core::new(config::gnosis_mainnet())), None);
        let inviter = address!("4000000000000000000000000000000000000004");

        let err = runtime
            .block_on(service.list_referrals(inviter, None, None))
            .expect_err("missing referrals backend");

        match err {
            SdkError::OperationFailed(message) => {
                assert!(message.contains("Referrals service not configured"))
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
