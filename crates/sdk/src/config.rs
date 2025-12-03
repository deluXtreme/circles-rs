use alloy_primitives::address;
use circles_types::CirclesConfig;
use once_cell::sync::Lazy;

/// Gnosis Chain (100) mainnet Circles configuration.
pub static GNOSIS_MAINNET: Lazy<CirclesConfig> = Lazy::new(|| CirclesConfig {
    circles_rpc_url: "https://rpc.aboutcircles.com/".to_string(),
    pathfinder_url: "https://pathfinder.aboutcircles.com".to_string(),
    profile_service_url: "https://rpc.aboutcircles.com/profiles/".to_string(),
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
    referrals_module_address: address!("d6df7cc2c2db03ec91761f4469d8dbaac7e538c9"),
});

/// Convenience helper to retrieve a cloned mainnet config.
pub fn gnosis_mainnet() -> CirclesConfig {
    GNOSIS_MAINNET.clone()
}
