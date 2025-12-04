use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::SolCall;
use async_trait::async_trait;
use thiserror::Error;

/// Prepared transaction for a runner to submit. This is intentionally simple;
/// we can swap to richer contract-specific types as we wire more flows.
#[derive(Debug, Clone)]
pub struct PreparedTransaction {
    pub to: Address,
    pub data: Bytes,
    pub value: Option<U256>,
}

/// Helper to turn a SolCall into a prepared transaction.
pub fn call_to_tx<C: SolCall>(to: Address, call: C, value: Option<U256>) -> PreparedTransaction {
    PreparedTransaction {
        to,
        data: Bytes::from(call.abi_encode()),
        value,
    }
}

/// Result stub for submitted transactions. Will be expanded when wiring a Safe runner.
#[derive(Debug, Clone)]
pub struct SubmittedTx {
    pub tx_hash: Bytes,
}

/// Trait that allows the SDK to send transactions (e.g., via a Safe).
#[async_trait]
pub trait ContractRunner: Send + Sync {
    /// Address of the sender/safe/owner associated with this runner.
    fn sender_address(&self) -> Address;

    /// Submit one or more prepared transactions.
    async fn send_transactions(
        &self,
        txs: Vec<PreparedTransaction>,
    ) -> Result<Vec<SubmittedTx>, RunnerError>;
}

/// Errors surfaced by the runner.
#[derive(Debug, Error)]
pub enum RunnerError {
    #[error("runner refused to send transactions: {0}")]
    Rejected(String),
    #[error("runner transport error: {0}")]
    Transport(String),
}
