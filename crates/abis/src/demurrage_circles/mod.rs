use alloy_sol_types::sol;

sol!(
    #[allow(clippy::too_many_arguments)]
    #[sol(rpc)]
    DemurrageCircles,
    "src/demurrage_circles/demurrage_circles.json"
);
