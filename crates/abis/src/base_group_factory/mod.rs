use alloy_sol_types::sol;

sol!(
    #[allow(clippy::too_many_arguments)]
    #[sol(rpc)]
    BaseGroupFactory,
    "src/base_group_factory/base_group_factory.json"
);
