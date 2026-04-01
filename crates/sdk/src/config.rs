use alloy_primitives::address;
use circles_types::CirclesConfig;
use once_cell::sync::Lazy;

/// Gnosis Chain (100) mainnet Circles configuration.
pub static GNOSIS_MAINNET: Lazy<CirclesConfig> = Lazy::new(|| CirclesConfig {
    circles_rpc_url: "https://rpc.aboutcircles.com/".to_string(),
    chain_rpc_url: None,
    pathfinder_url: None,
    profile_service_url: None,
    referrals_service_url: Some("https://referrals.aboutcircles.com".to_string()),
    v1_hub_address: address!("29b9a7fbb8995b2423a71cc17cf9810798f6c543"),
    v2_hub_address: address!("c12c1e50abb450d6205ea2c3fa861b3b834d13e8"),
    name_registry_address: address!("a27566fd89162cc3d40cb59c87aaaa49b85f3474"),
    base_group_mint_policy: address!("cca27c26cf7bac2a9928f42201d48220f0e3a549"),
    standard_treasury: address!("08f90ab73a515308f03a718257ff9887ed330c6e"),
    core_members_group_deployer: address!("feca40eb02fb1f4f5f795fc7a03c1a27819b1ded"),
    base_group_factory_address: address!("d0b5bd9962197beac4cba24244ec3587f19bd06d"),
    lift_erc20_address: address!("5f99a795dd2743c36d63511f0d4bc667e6d3cdb5"),
    invitation_escrow_address: address!("8f8b74fa13eaaff4176d061a0f98ad5c8e19c903"),
    invitation_farm_address: address!("0000000000000000000000000000000000000000"),
    referrals_module_address: address!("12105a9b291af2abb0591001155a75949b062ce5"),
    invitation_module_address: address!("00738aca013b7b2e6cfe1690f0021c3182fa40b5"),
});

/// Convenience helper to retrieve a cloned mainnet config.
pub fn gnosis_mainnet() -> CirclesConfig {
    GNOSIS_MAINNET.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gnosis_mainnet_matches_invitation_config_snapshot() {
        let config = gnosis_mainnet();

        assert_eq!(
            config.effective_chain_rpc_url(),
            "https://rpc.aboutcircles.com/"
        );
        assert_eq!(
            config.effective_profile_service_url(),
            "https://rpc.aboutcircles.com/profiles/"
        );
        assert_eq!(
            config.referrals_service_url.as_deref(),
            Some("https://referrals.aboutcircles.com")
        );
        assert_eq!(
            config.referrals_module_address,
            address!("12105a9b291af2abb0591001155a75949b062ce5")
        );
        assert_eq!(
            config.invitation_module_address,
            address!("00738aca013b7b2e6cfe1690f0021c3182fa40b5")
        );
    }
}
