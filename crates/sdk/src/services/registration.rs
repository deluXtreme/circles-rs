use crate::avatar::{BaseGroupAvatar, HumanAvatar, OrganisationAvatar};
use crate::cid_v0_to_digest::cid_v0_to_digest;
use crate::{RegistrationResult, Sdk, SdkError, call_to_tx};
use alloy_primitives::{Address, U256};
use circles_abis::{BaseGroupFactory, HubV2};
use circles_profiles::Profile;
use circles_types::AvatarInfo;

/// TS-style registration profile input: either a full profile to pin or an existing CID.
#[derive(Debug, Clone, Copy)]
pub enum RegistrationProfileInput<'a> {
    Profile(&'a Profile),
    Cid(&'a str),
}

impl<'a> From<&'a Profile> for RegistrationProfileInput<'a> {
    fn from(profile: &'a Profile) -> Self {
        Self::Profile(profile)
    }
}

impl<'a> From<&'a str> for RegistrationProfileInput<'a> {
    fn from(cid: &'a str) -> Self {
        Self::Cid(cid)
    }
}

/// Borrowed registration facade mirroring the TypeScript `sdk.register.*` namespace.
pub struct Registration<'a> {
    sdk: &'a Sdk,
}

impl<'a> Registration<'a> {
    pub(crate) fn new(sdk: &'a Sdk) -> Self {
        Self { sdk }
    }

    /// Register a human using either a profile to pin or an existing profile CID.
    pub async fn as_human<'p, P>(
        &self,
        inviter: Address,
        profile: P,
    ) -> Result<RegistrationResult<HumanAvatar>, SdkError>
    where
        P: Into<RegistrationProfileInput<'p>>,
    {
        register_human_with_profile_input(self.sdk, inviter, profile.into()).await
    }

    /// Register an organisation using either a profile to pin or an existing profile CID.
    ///
    /// When a CID is supplied, the profile is fetched first so the organisation name
    /// can be derived the same way the TypeScript SDK does.
    pub async fn as_organization<'p, P>(
        &self,
        profile: P,
    ) -> Result<RegistrationResult<OrganisationAvatar>, SdkError>
    where
        P: Into<RegistrationProfileInput<'p>>,
    {
        let (name, cid) = resolve_organization_name_and_cid(self.sdk, profile.into()).await?;
        submit_organisation_registration(self.sdk, name, cid).await
    }

    /// Register a base group using either a profile to pin or an existing profile CID.
    #[allow(clippy::too_many_arguments)]
    pub async fn as_group<'p, P>(
        &self,
        owner: Address,
        service: Address,
        fee_collection: Address,
        initial_conditions: &[Address],
        name: &str,
        symbol: &str,
        profile: P,
    ) -> Result<RegistrationResult<BaseGroupAvatar>, SdkError>
    where
        P: Into<RegistrationProfileInput<'p>>,
    {
        register_group_with_profile_input(
            self.sdk,
            owner,
            service,
            fee_collection,
            initial_conditions,
            name,
            symbol,
            profile.into(),
        )
        .await
    }
}

async fn resolve_profile_cid(
    sdk: &Sdk,
    profile: RegistrationProfileInput<'_>,
) -> Result<String, SdkError> {
    match profile {
        RegistrationProfileInput::Profile(profile) => Ok(sdk.profiles.create(profile).await?),
        RegistrationProfileInput::Cid(cid) => Ok(cid.to_owned()),
    }
}

async fn resolve_organization_name_and_cid(
    sdk: &Sdk,
    profile: RegistrationProfileInput<'_>,
) -> Result<(String, String), SdkError> {
    match profile {
        RegistrationProfileInput::Profile(profile) => {
            if profile.name.is_empty() {
                return Err(SdkError::InvalidRegistration(
                    "organisation name cannot be empty".to_string(),
                ));
            }
            Ok((profile.name.clone(), sdk.profiles.create(profile).await?))
        }
        RegistrationProfileInput::Cid(cid) => {
            let profile = sdk.profiles.get(cid).await?.ok_or_else(|| {
                SdkError::InvalidRegistration(format!("profile not found for cid {cid}"))
            })?;
            if profile.name.is_empty() {
                return Err(SdkError::InvalidRegistration(
                    "organisation name cannot be empty".to_string(),
                ));
            }
            Ok((profile.name, cid.to_owned()))
        }
    }
}

async fn submit_human_registration(
    sdk: &Sdk,
    inviter: Address,
    cid: &str,
) -> Result<RegistrationResult<HumanAvatar>, SdkError> {
    let runner = sdk.runner.clone().ok_or(SdkError::MissingRunner)?;
    let sender = sdk.sender_address.ok_or(SdkError::MissingSender)?;

    let inviters = sdk
        .core
        .invitation_escrow()
        .getInviters(sender)
        .call()
        .await
        .unwrap_or_default();

    let mut txs = Vec::new();
    if let Some(first_inviter) = inviters.first() {
        let redeem = circles_abis::InvitationEscrow::redeemInvitationCall {
            inviter: *first_inviter,
        };
        txs.push(call_to_tx(
            sdk.config.invitation_escrow_address,
            redeem,
            None,
        ));
    } else {
        let token_id = sdk
            .core
            .hub_v2()
            .toTokenId(inviter)
            .call()
            .await
            .map_err(|e| SdkError::InvalidRegistration(e.to_string()))?;
        let balance = sdk
            .core
            .hub_v2()
            .balanceOf(inviter, token_id)
            .call()
            .await
            .map_err(|e| SdkError::InvalidRegistration(e.to_string()))?;
        let min_required = U256::from(96u128) * U256::from(10).pow(U256::from(18));
        if balance < min_required {
            return Err(SdkError::InvalidRegistration(
                "inviter has insufficient balance (requires 96 CRC)".to_string(),
            ));
        }
    }

    let digest = cid_v0_to_digest(cid)?;
    let call = HubV2::registerHumanCall {
        _inviter: inviter,
        _metadataDigest: digest,
    };
    txs.push(call_to_tx(sdk.config.v2_hub_address, call, None));
    let sent = runner.send_transactions(txs).await?;
    let info = sdk.rpc.avatar().get_avatar_info(sender).await?;
    let avatar = HumanAvatar::new(
        sender,
        info,
        sdk.core.clone(),
        sdk.profiles.clone(),
        sdk.rpc.clone(),
        sdk.runner.clone(),
    );
    Ok(RegistrationResult {
        avatar: Some(avatar),
        txs: sent,
    })
}

async fn submit_organisation_registration(
    sdk: &Sdk,
    name: String,
    cid: String,
) -> Result<RegistrationResult<OrganisationAvatar>, SdkError> {
    if name.is_empty() {
        return Err(SdkError::InvalidRegistration(
            "organisation name cannot be empty".to_string(),
        ));
    }

    let runner = sdk.runner.clone().ok_or(SdkError::MissingRunner)?;
    let sender = sdk.sender_address.ok_or(SdkError::MissingSender)?;
    let digest = cid_v0_to_digest(&cid)?;
    let call = HubV2::registerOrganizationCall {
        _name: name,
        _metadataDigest: digest,
    };
    let txs = vec![call_to_tx(sdk.config.v2_hub_address, call, None)];
    let sent = runner.send_transactions(txs).await?;
    let info = sdk.rpc.avatar().get_avatar_info(sender).await?;
    let avatar = OrganisationAvatar::new(
        sender,
        info,
        sdk.core.clone(),
        sdk.profiles.clone(),
        sdk.rpc.clone(),
        sdk.runner.clone(),
    );
    Ok(RegistrationResult {
        avatar: Some(avatar),
        txs: sent,
    })
}

#[allow(clippy::too_many_arguments)]
async fn submit_group_registration(
    sdk: &Sdk,
    owner: Address,
    service: Address,
    fee_collection: Address,
    initial_conditions: &[Address],
    name: &str,
    symbol: &str,
    cid: &str,
) -> Result<RegistrationResult<BaseGroupAvatar>, SdkError> {
    if name.is_empty() || name.len() > 19 {
        return Err(SdkError::InvalidRegistration(
            "group name must be 1–19 chars".to_string(),
        ));
    }
    if symbol.is_empty() {
        return Err(SdkError::InvalidRegistration(
            "group symbol cannot be empty".to_string(),
        ));
    }

    let runner = sdk.runner.clone().ok_or(SdkError::MissingRunner)?;
    let digest = cid_v0_to_digest(cid)?;
    let call = BaseGroupFactory::createBaseGroupCall {
        _owner: owner,
        _service: service,
        _feeCollection: fee_collection,
        _initialConditions: initial_conditions.to_vec(),
        _name: name.to_string(),
        _symbol: symbol.to_string(),
        _metadataDigest: digest,
    };
    let txs = vec![call_to_tx(
        sdk.config.base_group_factory_address,
        call,
        None,
    )];
    let predicted = sdk
        .core
        .base_group_factory()
        .createBaseGroup(
            owner,
            service,
            fee_collection,
            initial_conditions.to_vec(),
            name.to_string(),
            symbol.to_string(),
            digest,
        )
        .call()
        .await
        .ok();

    let sent = runner.send_transactions(txs).await?;
    let avatar = if let Some(predicted) = predicted {
        let info: AvatarInfo = sdk.rpc.avatar().get_avatar_info(predicted.group).await?;
        Some(BaseGroupAvatar::new(
            predicted.group,
            info,
            sdk.core.clone(),
            sdk.profiles.clone(),
            sdk.rpc.clone(),
            sdk.runner.clone(),
        ))
    } else {
        None
    };

    Ok(RegistrationResult { avatar, txs: sent })
}

async fn register_human_with_profile_input(
    sdk: &Sdk,
    inviter: Address,
    profile: RegistrationProfileInput<'_>,
) -> Result<RegistrationResult<HumanAvatar>, SdkError> {
    let cid = resolve_profile_cid(sdk, profile).await?;
    submit_human_registration(sdk, inviter, &cid).await
}

#[allow(clippy::too_many_arguments)]
async fn register_group_with_profile_input(
    sdk: &Sdk,
    owner: Address,
    service: Address,
    fee_collection: Address,
    initial_conditions: &[Address],
    name: &str,
    symbol: &str,
    profile: RegistrationProfileInput<'_>,
) -> Result<RegistrationResult<BaseGroupAvatar>, SdkError> {
    let cid = resolve_profile_cid(sdk, profile).await?;
    submit_group_registration(
        sdk,
        owner,
        service,
        fee_collection,
        initial_conditions,
        name,
        symbol,
        &cid,
    )
    .await
}

/// Register a human avatar.
pub async fn register_human(
    sdk: &Sdk,
    inviter: Address,
    profile: &Profile,
) -> Result<RegistrationResult<HumanAvatar>, SdkError> {
    register_human_with_profile_input(sdk, inviter, profile.into()).await
}

/// Register an organisation avatar.
pub async fn register_organisation(
    sdk: &Sdk,
    name: &str,
    profile: &Profile,
) -> Result<RegistrationResult<OrganisationAvatar>, SdkError> {
    let cid = resolve_profile_cid(sdk, profile.into()).await?;
    submit_organisation_registration(sdk, name.to_string(), cid).await
}

#[allow(clippy::too_many_arguments)]
/// Register a base group via the factory (requires runner).
pub async fn register_group(
    sdk: &Sdk,
    owner: Address,
    service: Address,
    fee_collection: Address,
    initial_conditions: &[Address],
    name: &str,
    symbol: &str,
    profile: &Profile,
) -> Result<RegistrationResult<BaseGroupAvatar>, SdkError> {
    register_group_with_profile_input(
        sdk,
        owner,
        service,
        fee_collection,
        initial_conditions,
        name,
        symbol,
        profile.into(),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::{RegistrationProfileInput, register_human};
    use crate::{SdkError, config};
    use alloy_primitives::address;
    use circles_profiles::Profile;

    fn test_profile(name: &str) -> Profile {
        Profile {
            name: name.to_string(),
            description: None,
            preview_image_url: None,
            image_url: None,
            location: None,
            geo_location: None,
            extensions: None,
        }
    }

    #[test]
    fn profile_input_from_profile_and_cid_preserves_variant() {
        let profile = test_profile("Alice");
        assert!(matches!(
            RegistrationProfileInput::from(&profile),
            RegistrationProfileInput::Profile(_)
        ));
        assert!(matches!(
            RegistrationProfileInput::from("QmCid"),
            RegistrationProfileInput::Cid("QmCid")
        ));
    }

    #[tokio::test]
    async fn register_human_requires_runner_before_network_profile_work() {
        let sdk = crate::Sdk::new(config::gnosis_mainnet(), None).expect("sdk");
        let err = match register_human(
            &sdk,
            address!("1000000000000000000000000000000000000001"),
            &test_profile("Alice"),
        )
        .await
        {
            Ok(_) => panic!("missing runner"),
            Err(err) => err,
        };

        assert!(matches!(err, SdkError::MissingRunner));
    }
}
