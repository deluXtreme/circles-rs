use abis::{
    BaseGroup, BaseGroupFactory, DemurrageCircles, HubV2, InflationaryCircles, InvitationEscrow,
    InvitationFarm, LiftERC20, NameRegistry, ReferralsModule,
};
use alloy_provider::{Identity, ProviderBuilder, RootProvider};
use circles_types::CirclesConfig;

/// Core contract bundle for the Circles SDK.
#[derive(Clone)]
pub struct Core {
    pub config: CirclesConfig,
}

impl Core {
    pub fn new(config: CirclesConfig) -> Self {
        Self { config }
    }

    /// HTTP provider built from the configured Circles RPC URL.
    pub fn provider(&self) -> RootProvider {
        ProviderBuilder::<Identity, Identity>::default().connect_http(
            self.config
                .circles_rpc_url
                .parse()
                .expect("circles_rpc_url must be a valid URL"),
        )
    }

    pub fn hub_v2(&self) -> HubV2::HubV2Instance<RootProvider> {
        HubV2::new(self.config.v2_hub_address, self.provider())
    }

    pub fn name_registry(&self) -> NameRegistry::NameRegistryInstance<RootProvider> {
        NameRegistry::new(self.config.name_registry_address, self.provider())
    }

    pub fn base_group_factory(&self) -> BaseGroupFactory::BaseGroupFactoryInstance<RootProvider> {
        BaseGroupFactory::new(self.config.base_group_factory_address, self.provider())
    }

    pub fn base_group(
        &self,
        address: alloy_primitives::Address,
    ) -> BaseGroup::BaseGroupInstance<RootProvider> {
        BaseGroup::new(address, self.provider())
    }

    pub fn invitation_escrow(&self) -> InvitationEscrow::InvitationEscrowInstance<RootProvider> {
        InvitationEscrow::new(self.config.invitation_escrow_address, self.provider())
    }

    pub fn invitation_farm(&self) -> InvitationFarm::InvitationFarmInstance<RootProvider> {
        InvitationFarm::new(self.config.invitation_farm_address, self.provider())
    }

    pub fn lift_erc20(&self) -> LiftERC20::LiftERC20Instance<RootProvider> {
        LiftERC20::new(self.config.lift_erc20_address, self.provider())
    }

    pub fn demurrage_circles(&self) -> DemurrageCircles::DemurrageCirclesInstance<RootProvider> {
        DemurrageCircles::new(self.config.v2_hub_address, self.provider())
    }

    pub fn inflationary_circles(
        &self,
    ) -> InflationaryCircles::InflationaryCirclesInstance<RootProvider> {
        InflationaryCircles::new(self.config.v2_hub_address, self.provider())
    }

    pub fn referrals_module(&self) -> ReferralsModule::ReferralsModuleInstance<RootProvider> {
        ReferralsModule::new(self.config.referrals_module_address, self.provider())
    }
}
