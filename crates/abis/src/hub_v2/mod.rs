use alloy_sol_types::sol;

sol!(
    #[allow(clippy::too_many_arguments)]
    #[sol(rpc)]
    HubV2,
    "src/hub_v2/hub_v2.json"
);
