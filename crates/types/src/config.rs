use alloy_primitives::Address;
use serde::{Deserialize, Serialize};

/// Circles protocol configuration for a specific chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CirclesConfig {
    /// RPC URL for Circles-specific endpoints
    pub circles_rpc_url: String,
    /// Optional RPC URL for direct chain calls such as `eth_call`.
    ///
    /// Falls back to `circles_rpc_url` when omitted.
    pub chain_rpc_url: Option<String>,
    /// Deprecated pathfinder URL.
    ///
    /// Newer deployments serve pathfinder data from the main Circles RPC host.
    pub pathfinder_url: Option<String>,
    /// Optional explicit profile service URL.
    ///
    /// Falls back to `circles_rpc_url + "profiles/"` when omitted.
    pub profile_service_url: Option<String>,
    /// Optional referrals service URL for storing and retrieving referral metadata
    pub referrals_service_url: Option<String>,
    /// Circles V1 Hub contract address
    pub v1_hub_address: Address,
    /// Circles V2 Hub contract address
    pub v2_hub_address: Address,
    /// Name Registry contract address
    pub name_registry_address: Address,
    /// Base Group Mint Policy contract address
    pub base_group_mint_policy: Address,
    /// Standard Treasury contract address
    pub standard_treasury: Address,
    /// Core Members Group Deployer contract address
    pub core_members_group_deployer: Address,
    /// Base Group Factory contract address
    pub base_group_factory_address: Address,
    /// Lift ERC20 contract address
    pub lift_erc20_address: Address,
    /// Invitation Escrow contract address
    pub invitation_escrow_address: Address,
    /// Invitation Farm contract address
    pub invitation_farm_address: Address,
    /// Referrals Module contract address
    pub referrals_module_address: Address,
    /// Invitation Module contract address
    pub invitation_module_address: Address,
}

impl CirclesConfig {
    /// Chain RPC URL, falling back to `circles_rpc_url`.
    pub fn effective_chain_rpc_url(&self) -> &str {
        self.chain_rpc_url
            .as_deref()
            .unwrap_or(self.circles_rpc_url.as_str())
    }

    /// Profile service URL, falling back to `<circles_rpc_url>/profiles/`.
    pub fn effective_profile_service_url(&self) -> String {
        match self.profile_service_url.as_deref() {
            Some(url) => ensure_trailing_slash(url),
            None => format!("{}profiles/", ensure_trailing_slash(&self.circles_rpc_url)),
        }
    }
}

fn ensure_trailing_slash(url: &str) -> String {
    if url.ends_with('/') {
        url.to_owned()
    } else {
        format!("{url}/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_config() -> CirclesConfig {
        CirclesConfig {
            circles_rpc_url: "https://rpc.example.com".into(),
            chain_rpc_url: None,
            pathfinder_url: None,
            profile_service_url: None,
            referrals_service_url: None,
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
            invitation_module_address: Address::ZERO,
        }
    }

    #[test]
    fn effective_chain_rpc_url_defaults_to_circles_rpc_url() {
        let config = demo_config();
        assert_eq!(config.effective_chain_rpc_url(), "https://rpc.example.com");
    }

    #[test]
    fn effective_chain_rpc_url_uses_override_when_present() {
        let mut config = demo_config();
        config.chain_rpc_url = Some("https://chain.example.com".into());

        assert_eq!(
            config.effective_chain_rpc_url(),
            "https://chain.example.com"
        );
    }

    #[test]
    fn effective_profile_service_url_defaults_to_profiles_path() {
        let config = demo_config();
        assert_eq!(
            config.effective_profile_service_url(),
            "https://rpc.example.com/profiles/"
        );
    }

    #[test]
    fn effective_profile_service_url_normalizes_explicit_override() {
        let mut config = demo_config();
        config.profile_service_url = Some("https://profiles.example.com/api".into());

        assert_eq!(
            config.effective_profile_service_url(),
            "https://profiles.example.com/api/"
        );
    }
}
