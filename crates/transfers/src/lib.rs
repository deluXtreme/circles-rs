//! Circles transfer transaction builder.
//!
//! This crate mirrors the TypeScript `@aboutcircles/sdk-transfers` package: it
//! builds the sequence of transactions (unwraps, approvals, operateFlowMatrix,
//! re-wraps) without executing them. Implementation is incremental; the API is
//! in place for higher-level integration.

mod builder;
mod error;

pub use builder::{TransferBuilder, TransferTx};
pub use error::{TransferError, TransfersErrorSource};
