use alloy_primitives::{Address, TxHash};
use circles_types::{AvatarInfo, AvatarType, CirclesConfig};

fn dummy_config() -> CirclesConfig {
    CirclesConfig {
        circles_rpc_url: "https://rpc.example.com".into(),
        pathfinder_url: "".into(),
        profile_service_url: "https://profiles.example.com".into(),
        v1_hub_address: Address::ZERO,
        v2_hub_address: Address::ZERO,
        name_registry_address: Address::ZERO,
        base_group_mint_policy: Address::ZERO,
        standard_treasury: Address::ZERO,
        core_members_group_deployer: Address::ZERO,
        base_group_factory_address: Address::ZERO,
        lift_erc20_address: Address::ZERO,
        invitation_escrow_address: Address::ZERO,
        invitation_farm_address: Address::ZERO,
        referrals_module_address: Address::ZERO,
    }
}

fn dummy_avatar(address: Address) -> AvatarInfo {
    AvatarInfo {
        block_number: 0,
        timestamp: None,
        transaction_index: 0,
        log_index: 0,
        transaction_hash: TxHash::ZERO,
        version: 2,
        avatar_type: AvatarType::CrcV2RegisterHuman,
        avatar: address,
        token_id: None,
        has_v1: false,
        v1_token: None,
        cid_v0_digest: None,
        cid_v0: None,
        v1_stopped: None,
        is_human: true,
        name: None,
        symbol: None,
    }
}

#[test]
fn registration_helper_types_compile() {
    // This test ensures the registration helper path is accessible without running async.
    // (Full integration would require contract/mocks; this keeps compile-time coverage.)
    let config = dummy_config();
    let _info = dummy_avatar(Address::ZERO);
    // No runtime assertions; compilation is the check.
    let _ = config;
}
