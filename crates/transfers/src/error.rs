use alloy_primitives::Address;
use thiserror::Error;

/// Transfers package error source categories.
#[derive(Debug, Clone, Copy)]
pub enum TransfersErrorSource {
    Transfers,
    Pathfinding,
    FlowMatrix,
    Validation,
}

/// Transfer-specific errors.
#[derive(Debug, Error)]
pub enum TransferError {
    /// Generic error with optional context.
    #[error("{message}")]
    Generic {
        message: String,
        code: Option<String>,
        category: TransfersErrorSource,
    },
    /// No valid path found for the route.
    #[error("No valid transfer path found from {from:#x} to {to:#x}. {reason}")]
    NoPathFound {
        from: Address,
        to: Address,
        reason: String,
    },
    /// Insufficient balance for requested transfer.
    #[error("Insufficient balance for transfer. Requested: {requested} wei, available: {available} wei.")]
    InsufficientBalance {
        requested: String,
        available: String,
        from: Address,
        to: Address,
    },
    /// Wrapped tokens required but not enabled.
    #[error("Insufficient unwrapped token balance; wrapped tokens present but use_wrapped_balances is false.")]
    WrappedTokensRequired,
    /// Flow matrix contains unregistered avatars.
    #[error("Flow matrix contains {count} unregistered avatar(s): {addresses:?}")]
    UnregisteredAvatars {
        addresses: Vec<Address>,
        count: usize,
    },
    /// Flow matrix terminal sum mismatch.
    #[error(
        "Flow matrix terminal sum ({terminal_sum}) does not equal expected amount ({expected})"
    )]
    FlowMatrixMismatch {
        terminal_sum: String,
        expected: String,
    },
    /// Transfer path is empty.
    #[error("Transfer path is empty for route from {from:#x} to {to:#x}")]
    EmptyPath { from: Address, to: Address },
}

impl TransferError {
    pub fn no_path_found(from: Address, to: Address, reason: Option<String>) -> Self {
        TransferError::NoPathFound {
            from,
            to,
            reason: reason.unwrap_or_else(|| "This could mean there's no trust connection, insufficient balance, or the tokens are not transferable.".to_string()),
        }
    }

    pub fn insufficient_balance(
        requested: alloy_primitives::U256,
        available: alloy_primitives::U256,
        from: Address,
        to: Address,
    ) -> Self {
        TransferError::InsufficientBalance {
            requested: requested.to_string(),
            available: available.to_string(),
            from,
            to,
        }
    }

    pub fn wrapped_tokens_required() -> Self {
        TransferError::WrappedTokensRequired
    }

    pub fn unregistered_avatars(addresses: Vec<Address>) -> Self {
        let count = addresses.len();
        TransferError::UnregisteredAvatars { addresses, count }
    }

    pub fn flow_matrix_mismatch(
        terminal_sum: alloy_primitives::U256,
        expected: alloy_primitives::U256,
    ) -> Self {
        TransferError::FlowMatrixMismatch {
            terminal_sum: terminal_sum.to_string(),
            expected: expected.to_string(),
        }
    }

    pub fn empty_path(from: Address, to: Address) -> Self {
        TransferError::EmptyPath { from, to }
    }

    pub fn generic(
        message: impl Into<String>,
        code: Option<impl Into<String>>,
        category: TransfersErrorSource,
    ) -> Self {
        TransferError::Generic {
            message: message.into(),
            code: code.map(|c| c.into()),
            category,
        }
    }
}
