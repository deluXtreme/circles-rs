use alloy_sol_types::sol;

sol!(
    #[allow(clippy::too_many_arguments)]
    #[sol(rpc)]
    InvitationEscrow,
    "src/invitation_escrow/invitation_escrow.json"
);
