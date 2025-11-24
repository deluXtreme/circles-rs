use alloy_primitives::Address;
use serde::{Deserialize, Serialize};

/// Circles protocol configuration for a specific chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CirclesConfig {
    /// RPC URL for Circles-specific endpoints
    pub circles_rpc_url: String,
    /// Pathfinder service URL for computing transfer paths
    pub pathfinder_url: String,
    /// Profile service URL for user profiles and metadata
    pub profile_service_url: String,
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
}
