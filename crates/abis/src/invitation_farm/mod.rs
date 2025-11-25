use alloy_sol_types::sol;

sol!(
    #[allow(clippy::too_many_arguments)]
    #[sol(rpc)]
    InvitationFarm,
    "src/invitation_farm/invitation_farm.json"
);
