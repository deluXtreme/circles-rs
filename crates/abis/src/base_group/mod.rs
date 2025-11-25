use alloy_sol_types::sol;

sol!(
    #[allow(clippy::too_many_arguments)]
    #[sol(rpc)]
    BaseGroup,
    "src/base_group/base_group.json"
);
