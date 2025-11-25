use alloy_sol_types::sol;

sol!(
    #[allow(clippy::too_many_arguments)]
    #[sol(rpc)]
    ReferralsModule,
    "src/referrals_module/referrals_module.json"
);
