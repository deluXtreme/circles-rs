use alloy_sol_types::sol;

sol!(
    #[sol(rpc)]
    InvitationEscrow,
    "src/invitation_escrow/invitation_escrow.json"
);
