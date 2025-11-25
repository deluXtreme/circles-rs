use alloy_sol_types::sol;

sol!(
    #[allow(clippy::too_many_arguments)]
    #[sol(rpc)]
    NameRegistry,
    "src/name_registry/name_registry.json"
);
