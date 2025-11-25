use alloy_sol_types::sol;

sol!(
    #[allow(clippy::too_many_arguments)]
    #[sol(rpc)]
    InflationaryCircles,
    "src/inflationary_circles/inflationary_circles.json"
);
