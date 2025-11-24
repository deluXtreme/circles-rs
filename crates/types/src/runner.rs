use alloy_primitives::Address;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// Re-export from our base types
use crate::TransactionRequest;

/// Batch transaction runner trait
/// Allows multiple transactions to be batched and executed atomically
#[async_trait]
pub trait BatchRun {
    /// Transaction receipt type - using generic to avoid viem dependency
    type TransactionReceipt;
    /// Error type for batch operations
    type Error;

    /// Add a transaction to the batch
    fn add_transaction(&mut self, tx: TransactionRequest);

    /// Execute all batched transactions
    /// Returns single transaction receipt for the entire batch
    async fn run(self) -> Result<Self::TransactionReceipt, Self::Error>;
}

/// Contract runner trait for executing blockchain operations
/// This is the base trait that all contract runners must implement
#[async_trait]
pub trait ContractRunner {
    /// Public client type - using generic to avoid specific dependencies
    type PublicClient;
    /// Transaction receipt type
    type TransactionReceipt;
    /// Batch runner type
    type BatchRunner: BatchRun;
    /// Error type for operations
    type Error;

    /// The address of the account (if available)
    fn address(&self) -> Option<Address>;

    /// The public client for reading blockchain state
    fn public_client(&self) -> &Self::PublicClient;

    /// Initialize the runner
    async fn init(&mut self) -> Result<(), Self::Error>;

    /// Estimate gas for a transaction
    async fn estimate_gas(&self, tx: &TransactionRequest) -> Result<u64, Self::Error>;

    /// Call a contract (read-only)
    async fn call(&self, tx: &TransactionRequest) -> Result<String, Self::Error>;

    /// Resolve an ENS name to an address
    async fn resolve_name(&self, name: &str) -> Result<Option<Address>, Self::Error>;

    /// Send one or more transactions
    /// Safe: batches all transactions atomically and returns single TransactionReceipt
    async fn send_transaction(
        &self,
        txs: Vec<TransactionRequest>,
    ) -> Result<Self::TransactionReceipt, Self::Error>;

    /// Create a batch transaction runner (if supported)
    /// This allows multiple transactions to be executed atomically in a single on-chain transaction
    /// Typically used with Safe multisig or other smart contract wallets
    ///
    /// Returns a BatchRun instance for adding transactions and executing them as a batch
    fn send_batch_transaction(&self) -> Option<Self::BatchRunner>;
}

/// Configuration for contract runners
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerConfig {
    /// RPC endpoint URL
    pub rpc_url: String,
    /// Chain ID
    pub chain_id: u64,
    /// Default gas limit
    pub default_gas_limit: Option<u64>,
    /// Default gas price
    pub default_gas_price: Option<u64>,
}
