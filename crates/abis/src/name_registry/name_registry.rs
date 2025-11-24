use alloy_sol_types::sol;

sol!(
    #[sol(rpc)]
    NameRegistry,
    "src/name_registry/name_registry.json"
);
