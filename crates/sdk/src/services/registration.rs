use crate::avatar::{BaseGroupAvatar, HumanAvatar, OrganisationAvatar};
use crate::cid_v0_to_digest::cid_v0_to_digest;
use crate::{RegistrationResult, Sdk, SdkError, call_to_tx};
use alloy_primitives::{Address, U256};
use circles_abis::{BaseGroupFactory, HubV2};
use circles_profiles::Profile;
use circles_types::AvatarInfo;

/// Register a human avatar.
///
/// Flow:
/// - Uses runner/sender (errors if missing).
/// - If pending invitations exist in InvitationEscrow, redeems the first; else
///   checks inviter has ≥96 CRC and fails if not.
/// - Pins profile via profiles service and calls `HubV2::registerHuman`.
/// - Returns the created `HumanAvatar` and submitted txs.
pub async fn register_human(
    sdk: &Sdk,
    inviter: Address,
    profile: &Profile,
) -> Result<RegistrationResult<HumanAvatar>, SdkError> {
    let runner = sdk.runner.clone().ok_or(SdkError::MissingRunner)?;
    let sender = sdk.sender_address.ok_or(SdkError::MissingSender)?;

    // Check invitation escrow for pending invites; redeem first if present.
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
        // No pending invites; ensure inviter has at least 96 CRC.
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

    let cid = sdk.profiles.create(profile).await?;
    let digest = cid_v0_to_digest(&cid)?;
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

/// Register an organisation avatar.
///
/// Validates `name` non-empty, pins profile, calls `HubV2::registerOrganization`,
/// and returns the created `OrganisationAvatar` (requires runner).
pub async fn register_organisation(
    sdk: &Sdk,
    name: &str,
    profile: &Profile,
) -> Result<RegistrationResult<OrganisationAvatar>, SdkError> {
    if name.is_empty() {
        return Err(SdkError::InvalidRegistration(
            "organisation name cannot be empty".to_string(),
        ));
    }
    let runner = sdk.runner.clone().ok_or(SdkError::MissingRunner)?;
    let sender = sdk.sender_address.ok_or(SdkError::MissingSender)?;

    let cid = sdk.profiles.create(profile).await?;
    let digest = cid_v0_to_digest(&cid)?;
    let call = HubV2::registerOrganizationCall {
        _name: name.to_string(),
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
/// Register a base group via the factory (requires runner).
///
/// Validates name/symbol, pins profile, calls `BaseGroupFactory::createBaseGroup`,
/// and predicts the deployed address to return a `BaseGroupAvatar` when possible.
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

    let cid = sdk.profiles.create(profile).await?;
    let digest = cid_v0_to_digest(&cid)?;
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
