use alloy_sol_types::sol;

sol!(
    #[allow(clippy::too_many_arguments)]
    #[sol(rpc)]
    LiftERC20,
    "src/lift_erc20/lift_erc20.json"
);
