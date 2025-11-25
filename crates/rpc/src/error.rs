use alloy_provider::transport::TransportError;
use thiserror::Error;

/// Result alias for the Circles RPC crate.
pub type Result<T> = std::result::Result<T, CirclesRpcError>;

/// Top-level error type for Circles RPC operations.
#[derive(Debug, Error)]
pub enum CirclesRpcError {
    /// Underlying provider/transport error.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// (De)serialization issues.
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    /// Unexpected or malformed response payload.
    #[error("invalid response: {message}")]
    InvalidResponse { message: String },
    /// WebSocket subscription closed unexpectedly.
    #[error("subscription closed")]
    SubscriptionClosed,
}
