use alloy_primitives::U256;
use serde::{Deserialize, Serialize};

/// Escrowed Amount and Days Result
/// Returned by InvitationEscrow.getEscrowedAmountAndDays()
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscrowedAmountAndDays {
    pub escrowed_amount: U256,
    pub days_: U256,
}
