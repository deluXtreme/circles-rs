//! Runner abstractions and concrete wallet-backed implementations for write-capable SDK flows.
//!
//! The SDK prepares transactions as ABI-encoded calls and delegates submission to
//! an implementation of [`ContractRunner`]. This keeps the read path independent
//! from any wallet, Safe, or signer transport while still allowing a concrete
//! execution backend when the caller wants write parity.

use alloy_network::AnyNetwork;
use alloy_primitives::{Address, Bytes, U256, aliases::TxHash};
use alloy_provider::{Identity, ProviderBuilder, RootProvider};
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::SolCall;
use async_trait::async_trait;
use reqwest::Url;
use safe_rs::{
    CallBuilder, Eoa, EoaBatchResult, Error as SafeRsError, ExecutionResult, Safe, Wallet,
    WalletBuilder,
};
use thiserror::Error;

type AnyHttpProvider = RootProvider<AnyNetwork>;
type SafeWallet = Wallet<Safe<AnyHttpProvider>>;
type EoaWallet = Wallet<Eoa<AnyHttpProvider>>;

/// Prepared transaction for a runner to submit. This is intentionally simple;
/// we can swap to richer contract-specific types as we wire more flows.
#[derive(Debug, Clone)]
pub struct PreparedTransaction {
    /// Contract address to call.
    pub to: Address,
    /// ABI-encoded calldata.
    pub data: Bytes,
    /// Optional native value to send alongside the call.
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

/// Result stub for submitted transactions.
///
/// For Safe-backed execution this will usually contain a single item because the
/// full batch is submitted atomically as one on-chain Safe transaction.
#[derive(Debug, Clone)]
pub struct SubmittedTx {
    /// Runner-reported transaction hash bytes.
    pub tx_hash: Bytes,
}

/// Trait that allows the SDK to send transactions (e.g., via a Safe or EOA backend).
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

fn tx_hash_to_bytes(tx_hash: TxHash) -> Bytes {
    Bytes::copy_from_slice(tx_hash.as_slice())
}

fn submitted_from_execution_result(result: ExecutionResult) -> Vec<SubmittedTx> {
    vec![SubmittedTx {
        tx_hash: tx_hash_to_bytes(result.tx_hash),
    }]
}

fn submitted_from_eoa_result(result: EoaBatchResult) -> Vec<SubmittedTx> {
    result
        .results
        .into_iter()
        .map(|tx| SubmittedTx {
            tx_hash: tx_hash_to_bytes(tx.tx_hash),
        })
        .collect()
}

fn map_safe_error(error: SafeRsError) -> RunnerError {
    match error {
        SafeRsError::Provider(_) | SafeRsError::Fetch { .. } | SafeRsError::UnsupportedChain(_) => {
            RunnerError::Transport(error.to_string())
        }
        _ => RunnerError::Rejected(error.to_string()),
    }
}

fn parse_rpc_url(rpc_url: &str) -> Result<Url, RunnerError> {
    rpc_url
        .parse()
        .map_err(|err| RunnerError::Rejected(format!("invalid rpc url: {err}")))
}

fn parse_private_key(private_key: &str) -> Result<PrivateKeySigner, RunnerError> {
    private_key
        .parse()
        .map_err(|err| RunnerError::Rejected(format!("invalid private key: {err}")))
}

fn build_read_provider(rpc_url: Url) -> AnyHttpProvider {
    ProviderBuilder::<Identity, Identity, AnyNetwork>::default().connect_http(rpc_url)
}

fn attach_prepared_transactions<B>(builder: B, txs: Vec<PreparedTransaction>) -> B
where
    B: CallBuilder,
{
    txs.into_iter().fold(builder, |builder, tx| {
        builder.add_raw(tx.to, tx.value.unwrap_or_default(), tx.data)
    })
}

/// Safe-backed contract runner using `safe-rs`.
///
/// This currently targets single-owner (1/1 threshold) Safes, matching the
/// current capabilities of the underlying Safe crate used here.
pub struct SafeContractRunner {
    wallet: SafeWallet,
}

impl SafeContractRunner {
    /// Connect to an existing Safe using the given RPC URL, signer private key,
    /// and Safe address.
    pub async fn connect(
        rpc_url: &str,
        private_key: &str,
        safe_address: Address,
    ) -> Result<Self, RunnerError> {
        let rpc_url = parse_rpc_url(rpc_url)?;
        let signer = parse_private_key(private_key)?;
        let provider = build_read_provider(rpc_url);
        let wallet = WalletBuilder::new(provider, signer)
            .connect(safe_address)
            .await
            .map_err(map_safe_error)?;
        wallet
            .inner()
            .verify_single_owner()
            .await
            .map_err(map_safe_error)?;
        Ok(Self { wallet })
    }
}

#[async_trait]
impl ContractRunner for SafeContractRunner {
    fn sender_address(&self) -> Address {
        self.wallet.address()
    }

    async fn send_transactions(
        &self,
        txs: Vec<PreparedTransaction>,
    ) -> Result<Vec<SubmittedTx>, RunnerError> {
        if txs.is_empty() {
            return Err(RunnerError::Rejected(
                "no transactions provided".to_string(),
            ));
        }

        let result = attach_prepared_transactions(self.wallet.batch(), txs)
            .execute()
            .await
            .map_err(map_safe_error)?;
        Ok(submitted_from_execution_result(result))
    }
}

/// EOA-backed contract runner using the same call-builder model as the Safe runner.
///
/// Unlike Safe execution, multi-transaction batches are submitted sequentially
/// and therefore are not atomic.
pub struct EoaContractRunner {
    wallet: EoaWallet,
}

impl EoaContractRunner {
    /// Connect to an EOA signer using the given RPC URL and private key.
    pub async fn connect(rpc_url: &str, private_key: &str) -> Result<Self, RunnerError> {
        let rpc_url = parse_rpc_url(rpc_url)?;
        let signer = parse_private_key(private_key)?;
        let provider = build_read_provider(rpc_url.clone());
        let wallet = WalletBuilder::new(provider, signer)
            .connect_eoa(rpc_url)
            .await
            .map_err(map_safe_error)?;
        Ok(Self { wallet })
    }
}

#[async_trait]
impl ContractRunner for EoaContractRunner {
    fn sender_address(&self) -> Address {
        self.wallet.address()
    }

    async fn send_transactions(
        &self,
        txs: Vec<PreparedTransaction>,
    ) -> Result<Vec<SubmittedTx>, RunnerError> {
        if txs.is_empty() {
            return Err(RunnerError::Rejected(
                "no transactions provided".to_string(),
            ));
        }

        let result = attach_prepared_transactions(self.wallet.batch(), txs)
            .execute()
            .await
            .map_err(map_safe_error)?;
        Ok(submitted_from_eoa_result(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_node_bindings::Anvil;
    use alloy_primitives::address;
    use alloy_provider::Provider;
    use safe_rs::{Call, EoaTxResult, SimulationResult};

    const ANVIL_FIRST_PRIVATE_KEY: &str =
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
    const ANVIL_FIRST_ADDRESS: Address = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
    const ANVIL_SECOND_ADDRESS: Address = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");

    #[derive(Default)]
    struct StubBuilder {
        calls: Vec<Call>,
        simulation_result: Option<SimulationResult>,
    }

    impl CallBuilder for StubBuilder {
        fn calls_mut(&mut self) -> &mut Vec<Call> {
            &mut self.calls
        }

        fn calls(&self) -> &Vec<Call> {
            &self.calls
        }

        fn with_gas_limit(self, _gas_limit: u64) -> Self {
            self
        }

        async fn simulate(self) -> safe_rs::Result<Self> {
            Ok(self)
        }

        fn simulation_result(&self) -> Option<&SimulationResult> {
            self.simulation_result.as_ref()
        }

        fn simulation_success(self) -> safe_rs::Result<Self> {
            Ok(self)
        }
    }

    #[test]
    fn attach_prepared_transactions_preserves_order_and_default_value() {
        let txs = vec![
            PreparedTransaction {
                to: Address::repeat_byte(0x11),
                data: Bytes::from_static(&[0xaa, 0xbb]),
                value: None,
            },
            PreparedTransaction {
                to: Address::repeat_byte(0x22),
                data: Bytes::from_static(&[0xcc]),
                value: Some(U256::from(42u64)),
            },
        ];

        let builder = attach_prepared_transactions(StubBuilder::default(), txs);
        assert_eq!(builder.calls.len(), 2);
        assert_eq!(builder.calls[0].to, Address::repeat_byte(0x11));
        assert_eq!(builder.calls[0].value, U256::ZERO);
        assert_eq!(builder.calls[0].data, Bytes::from_static(&[0xaa, 0xbb]));
        assert_eq!(builder.calls[1].to, Address::repeat_byte(0x22));
        assert_eq!(builder.calls[1].value, U256::from(42u64));
        assert_eq!(builder.calls[1].data, Bytes::from_static(&[0xcc]));
    }

    #[test]
    fn submitted_from_execution_result_returns_single_hash() {
        let submitted = submitted_from_execution_result(ExecutionResult {
            tx_hash: TxHash::repeat_byte(0x44),
            success: true,
        });

        assert_eq!(submitted.len(), 1);
        assert_eq!(submitted[0].tx_hash, Bytes::copy_from_slice(&[0x44; 32]));
    }

    #[test]
    fn submitted_from_eoa_result_preserves_order() {
        let submitted = submitted_from_eoa_result(EoaBatchResult {
            results: vec![
                EoaTxResult {
                    tx_hash: TxHash::repeat_byte(0x01),
                    success: true,
                    index: 0,
                },
                EoaTxResult {
                    tx_hash: TxHash::repeat_byte(0x02),
                    success: true,
                    index: 1,
                },
            ],
            success_count: 2,
            failure_count: 0,
            first_failure: None,
        });

        assert_eq!(submitted.len(), 2);
        assert_eq!(submitted[0].tx_hash, Bytes::copy_from_slice(&[0x01; 32]));
        assert_eq!(submitted[1].tx_hash, Bytes::copy_from_slice(&[0x02; 32]));
    }

    #[tokio::test]
    async fn safe_runner_rejects_invalid_private_key() {
        let result = SafeContractRunner::connect(
            "https://rpc.example.invalid",
            "not-a-private-key",
            Address::ZERO,
        )
        .await;

        assert!(matches!(result, Err(RunnerError::Rejected(_))));
    }

    #[tokio::test]
    async fn eoa_runner_rejects_invalid_private_key() {
        let result = EoaContractRunner::connect("https://rpc.example.invalid", "bad-key").await;

        assert!(matches!(result, Err(RunnerError::Rejected(_))));
    }

    #[tokio::test]
    async fn eoa_runner_executes_value_transfer_on_anvil() {
        let anvil = Anvil::new().spawn();
        let provider = build_read_provider(anvil.endpoint_url());
        let before = provider
            .get_balance(ANVIL_SECOND_ADDRESS)
            .await
            .expect("balance before transfer");
        let runner = EoaContractRunner::connect(&anvil.endpoint(), ANVIL_FIRST_PRIVATE_KEY)
            .await
            .expect("connect EOA runner");

        assert_eq!(runner.sender_address(), ANVIL_FIRST_ADDRESS);

        let submitted = runner
            .send_transactions(vec![PreparedTransaction {
                to: ANVIL_SECOND_ADDRESS,
                data: Bytes::new(),
                value: Some(U256::from(123u64)),
            }])
            .await
            .expect("EOA transfer executes");

        assert_eq!(submitted.len(), 1);

        let after = provider
            .get_balance(ANVIL_SECOND_ADDRESS)
            .await
            .expect("balance after transfer");
        assert_eq!(after - before, U256::from(123u64));
    }

    #[tokio::test]
    async fn safe_runner_rejects_non_safe_address_on_plain_anvil() {
        let anvil = Anvil::new().spawn();
        let result = SafeContractRunner::connect(
            &anvil.endpoint(),
            ANVIL_FIRST_PRIVATE_KEY,
            ANVIL_FIRST_ADDRESS,
        )
        .await;

        assert!(matches!(result, Err(RunnerError::Transport(_))));
    }
}
